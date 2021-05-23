// Copyright (c) 2021 10X Genomics, Inc. All rights reserved.

// This is a text client, which is useless except for debugging and experimentation.
//
// This starts enclone_server, which can be either local or remote.
//
// For now accepts a single argument, which is COM=x where x is a "configuration name".  It then
// looks for an environment variable ENCLONE_COM.x, and parses that into blank-separated
// arguments, which may be:
//
// argument                  interpretation
//
// REMOTE_HOST=...           name of remote host for password-free ssh
// REMOTE_IP=...             IP number of remote host
// REMOTE_SETUP=...          command to be forked to use port through firewall, may include $port
// REMOVE_BIN=...            directory on remote host containing the enclone_server executable

use enclone_core::parse_bsv;
use enclone_server::proto::{analyzer_client::AnalyzerClient, ClonotypeRequest, EncloneRequest};
use itertools::Itertools;
use nix::sys::signal::{kill, SIGINT};
use nix::unistd::Pid;
use pretty_trace::*;
use rand::Rng;
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use string_utils::*;

fn truncate(s: &str) -> String {
    const MAX_LINES: usize = 10;
    let mut t = String::new();
    let mut extra = 0;
    for (i, line) in s.lines().enumerate() {
        if i < MAX_LINES {
            t += &mut format!("{}\n", line);
        } else {
            extra += 1;
        }
    }
    if extra > 0 {
        t += &mut format!("(+ {} more lines)", extra);
    }
    t
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    PrettyTrace::new().on();

    // Get configuration.

    let mut configuration = None;
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let arg = args[1].to_string();
        if !arg.starts_with("COM=") {
            eprintln!(
                "\nCurrently the only allowed argument is COM=x where x is a \
                configuration name.\n"
            );
            std::process::exit(1);
        }
        let config_name = arg.after("COM=");
        let env_var = format!("ENCLONE_COM.{}", config_name);
        for (key, value) in env::vars() {
            if key == env_var {
                configuration = Some(value.clone());
            }
        }
        if configuration.is_none() {
            eprintln!(
                "\nYou specified the configuration name {}, but the environment variable {} \
                is not defined.\n",
                config_name, env_var,
            );
            std::process::exit(1);
        }
    }
    let mut config = HashMap::<String, String>::new();
    if configuration.is_some() {
        let configuration = configuration.unwrap();
        let x = parse_bsv(&configuration);
        for arg in x.iter() {
            if !arg.contains("=") {
                eprintln!(
                    "\nYour configuration has an argument {} that does not contain =.\n",
                    arg
                );
                std::process::exit(1);
            }
            config.insert(arg.before("=").to_string(), arg.after("=").to_string());
        }
    }

    // Loop through random ports until we get one that works.

    let mut rng = rand::thread_rng();
    loop {
        let port: u16 = rng.gen();
        if port < 1024 {
            continue;
        }

        // Attempt to fork the server.

        let mut remote = false;
        let server_process;
        let mut local_host = "127.0.0.1".to_string();
        if config.contains_key("REMOTE_HOST")
            || config.contains_key("REMOTE_IP")
            || config.contains_key("REMOTE_BIN")
        {
            if !config.contains_key("REMOTE_HOST")
                || !config.contains_key("REMOTE_IP")
                || !config.contains_key("REMOTE_BIN")
            {
                eprintln!(
                    "\nTo use a remote host, please specify all of REMOTE_HOST, \
                    REMOTE_IP, and REMOTE_BIN.\n"
                );
                eprintln!("Here is what is specified:");
                for (key, value) in config.iter() {
                    eprintln!("{}={}", key, value);
                }
                std::process::exit(1);
            }
            remote = true;
        }
        println!("trying random port {}", port);
        let mut host = String::new();
        if remote {
            host = config["REMOTE_HOST"].clone();
            let ip = &config["REMOTE_IP"];
            let bin = &config["REMOTE_BIN"];
            server_process = Command::new("ssh")
                .arg(&host)
                .arg(&format!("{}/enclone_server", bin))
                .arg(&format!("{}:{}", ip, port))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            local_host = "local_host".to_string();
        } else {
            let ip = "127.0.0.1";
            server_process = Command::new("enclone_server")
                .arg(&format!("{}:{}", ip, port))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
        }
        if !server_process.is_ok() {
            eprintln!(
                "\nfailed to launch server, err =\n{}.\n",
                server_process.unwrap_err()
            );
            std::process::exit(1);
        }
        let server_process = server_process.unwrap();
        let server_process_id = server_process.id();

        // Wait until server has printed something.

        let mut buffer = [0; 50];
        let mut server_stdout = server_process.stdout.unwrap();
        server_stdout.read(&mut buffer).unwrap();
        thread::sleep(Duration::from_millis(100));

        // Look at stderr.

        let mut ebuffer = [0; 200];
        let mut server_stderr = server_process.stderr.unwrap();
        server_stderr.read(&mut ebuffer).unwrap();
        let emsg = strme(&ebuffer);
        if emsg.len() > 0 {
            if emsg.contains("already in use") {
                println!("oops, that port is in use, trying a different one");
                continue;
            }
            eprintln!("\nserver says this:\n{}", emsg);
        }

        // Get server process id, possibly remote.

        let mut remote_id = None;
        if remote {
            if emsg.contains("I am process ") && emsg.after("I am process ").contains(".") {
                let id = emsg.between("I am process ", ".");
                if id.parse::<usize>().is_ok() {
                    remote_id = Some(id.force_usize());
                }
            }
            if remote_id.is_none() {
                eprintln!("\nUnable to determine remote process id.\n");
                std::process::exit(1);
            }
        }

        // Fork remote setup command if needed.

        if config.contains_key("REMOTE_SETUP") {
            if !remote {
                eprintln!("\nYou specified REMOTE_SETUP but not REMOTE_HOST.\n");
                std::process::exit(1);
            }
            let mut setup = config["REMOTE_SETUP"].clone();
            if setup.starts_with("\"") && setup.ends_with("\"") {
                setup = setup.after("\"").rev_before("\"").to_string();
            }
            setup = setup.replace("$port", &format!("{}", port));
            let argsp = setup.split(' ').collect::<Vec<&str>>();
            let args = argsp[1..].to_vec();
            eprintln!("running setup command = {}", argsp.iter().format(" "));
            let setup_process = Command::new(argsp[0])
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            if !setup_process.is_ok() {
                eprintln!(
                    "\nfailed to launch setup, err =\n{}.\n",
                    setup_process.unwrap_err()
                );
                kill(Pid::from_raw(server_process_id as i32), SIGINT).unwrap();
                std::process::exit(1);
            }
            let _setup_process = setup_process.unwrap();
            thread::sleep(Duration::from_millis(100));
        }

        // Form local URL.  Not really a URL.

        let url = format!("{}:{}", local_host, port);

        // Connect to client.

        println!("connecting to {}", url);
        let mut client = AnalyzerClient::connect(url).await?;

        // Accept commands.

        loop {
            print!(
                "\nenter one of the following:\n\
                • an enclone command, without the enclone part\n\
                • an clonotype id (number)\n\
                • d, for a demo, same as BCR=123085 MIN_CELLS=5 PLOT_BY_ISOTYPE=gui\n\
                • q to quit\n\n% "
            );
            std::io::stdout().flush().unwrap();
            let stdin = io::stdin();
            let mut line = stdin.lock().lines().next().unwrap().unwrap();
            if line == "q" {
                println!("");
                break;
            }
            if line == "d" {
                line = "BCR=123085 MIN_CELLS=5 PLOT_BY_ISOTYPE=gui".to_string();
            }
            if line.parse::<usize>().is_ok() {
                let n = line.force_usize();
                let request = tonic::Request::new(ClonotypeRequest {
                    clonotype_number: n as u32,
                });
                let response = client.get_clonotype(request).await?;
                let r = response.into_inner();
                println!("\ntable = {}", truncate(&r.table));
            } else {
                let request = tonic::Request::new(EncloneRequest { args: line });
                let response = client.enclone(request).await?;
                let r = response.into_inner();
                println!("\nargs = {}", r.args);
                println!("\nplot = {}", truncate(&r.plot));
                println!("\ntable = {}", truncate(&r.table));
            }
        }

        // Kill server.

        kill(Pid::from_raw(server_process_id as i32), SIGINT).unwrap();
        if remote {
            Command::new("ssh")
                .arg(&host)
                .arg("kill")
                .arg("-9")
                .arg(&format!("{}", remote_id.unwrap()))
                .output()
                .expect("failed to execute ssh to kill");
        }

        // Done.

        return Ok(());
    }
}