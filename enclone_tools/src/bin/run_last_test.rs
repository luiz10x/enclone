// Copyright (c) 2021 10X Genomics, Inc. All rights reserved.

// Run just the last test in the TESTS category in testlist.rs.
//
// Note that this gives the wrong answer if you haven't run ./build first.

use enclone_core::main_testlist::*;
use enclone_tools::run_test::*;
use pretty_trace::*;

fn main() {
    PrettyTrace::new().on();
    println!(
        "\nYou should already have run ./build.  Otherwise this will give you the \
        wrong answer!\n"
    );
    let mut out = String::new();
    let mut ok = false;
    let mut logx = String::new();
    run_test(
        "enclone",
        TESTS.len() - 1,
        "",
        TESTS[TESTS.len() - 1],
        "test",
        &mut ok,
        &mut logx,
        &mut out,
    );
    print!("{}", logx);
}
