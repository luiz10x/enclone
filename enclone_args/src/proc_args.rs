// Copyright (c) 2021 10X Genomics, Inc. All rights reserved.

use crate::proc_args2::*;
use crate::proc_args_post::*;
use crate::process_special_arg::*;
use enclone_core::defs::*;
use enclone_core::*;
use io_utils::*;
use itertools::Itertools;
use std::fs::{remove_file, File};
use std::{process::Command, time::Instant};
use string_utils::*;
use tilde_expand::*;

// Process arguments.

pub fn proc_args(mut ctl: &mut EncloneControl, args: &Vec<String>) -> Result<(), String> {
    // Knobs.

    if ctl.gen_opt.evil_eye {
        println!("processing args");
    }
    let targs = Instant::now();
    let heur = ClonotypeHeuristics {
        max_diffs: 55,
        max_degradation: 2,
        ref_v_trim: 15,
        ref_j_trim: 15,
    };
    ctl.heur = heur;
    let mut args = args.clone();
    let mut args2 = Vec::<String>::new();
    for i in 0..args.len() {
        if args[i].starts_with("BCR_GEX=") {
            args2.push(format!("BCR={}", args[i].after("BCR_GEX=")));
            args2.push(format!("GEX={}", args[i].after("BCR_GEX=")));
        } else if args[i].starts_with("TCR_GEX=") {
            args2.push(format!("TCR={}", args[i].after("TCR_GEX=")));
            args2.push(format!("GEX={}", args[i].after("TCR_GEX=")));
        } else if args[i].starts_with("GD_BC=") {
            args2.push(format!(
                "BC={}/outs/genetic_demux_results/clusters.tsv",
                args[i].after("GD_BC=")
            ));
        } else {
            args2.push(args[i].clone());
        }
    }
    args = args2;

    // Process special option SPLIT_COMMAND.

    if ctl.gen_opt.evil_eye {
        println!("at split command");
    }
    if ctl.gen_opt.split {
        let (mut bcr, mut gex) = (Vec::<&str>::new(), Vec::<&str>::new());
        let mut args2 = Vec::<String>::new();
        for i in 1..args.len() {
            if args[i] == "SPLIT_BY_COMMAND" {
            } else if args[i].starts_with("BCR=") {
                bcr = args[i].after("BCR=").split(',').collect::<Vec<&str>>();
            } else if args[i].starts_with("GEX=") {
                gex = args[i].after("GEX=").split(',').collect::<Vec<&str>>();
            } else {
                args2.push(args[i].to_string());
            }
        }
        for i in 0..bcr.len() {
            let mut args = args2.clone();
            args.push(format!("BCR={}", bcr[i]));
            args.push(format!("GEX={}", gex[i]));
            println!("\nenclone {}\n", args.iter().format(" "));
            let o = Command::new("enclone")
                .args(&args)
                .output()
                .expect("failed to execute enclone");
            print!("{}{}", strme(&o.stdout), strme(&o.stderr));
            if o.status.code() != Some(0) {
                return Err("\nFAILED!\n".to_string());
            }
        }
        return Ok(());
    }

    // Set up general options.

    if ctl.gen_opt.evil_eye {
        println!("setting up general options");
    }
    ctl.gen_opt.h5_pre = true;
    ctl.gen_opt.min_cells_exact = 1;
    ctl.gen_opt.min_chains_exact = 1;
    ctl.gen_opt.exact = None;
    ctl.gen_opt.full_counts = true;
    ctl.gen_opt.color = "codon".to_string();
    ctl.silent = true;
    ctl.gen_opt.peer_group_dist = "MFL".to_string();
    ctl.gen_opt.color_by_rarity_pc = -1.0;
    ctl.gen_opt.jscore_match = 20;
    ctl.gen_opt.jscore_mismatch = -20;
    ctl.gen_opt.jscore_gap_open = -120;
    ctl.gen_opt.jscore_gap_extend = -20;
    ctl.gen_opt.jscore_bits_multiplier = 2.2;
    ctl.gen_opt.max_heavies = 1000000;

    // Set up clonotyping control parameters.

    ctl.clono_filt_opt.ncells_low = 1;
    ctl.clono_filt_opt.ncells_high = 1_000_000_000;
    ctl.clono_filt_opt.min_umi = 0;
    ctl.clono_filt_opt.max_chains = 1000000;
    ctl.clono_filt_opt.qual_filter = true;
    ctl.clono_filt_opt_def.signature = true;
    ctl.clono_filt_opt_def.weak_chains = true;
    ctl.clono_filt_opt_def.weak_onesies = true;
    ctl.clono_filt_opt_def.weak_foursies = true;
    ctl.clono_filt_opt_def.doublet = true;
    ctl.clono_filt_opt_def.bc_dup = true;
    ctl.clono_filt_opt.max_datasets = 1000000000;
    ctl.clono_filt_opt_def.umi_filt = true;
    ctl.clono_filt_opt_def.umi_ratio_filt = true;
    ctl.clono_filt_opt.max_exacts = 1_000_000_000;

    ctl.clono_print_opt.amino = vec![
        "cdr3".to_string(),
        "var".to_string(),
        "share".to_string(),
        "donor".to_string(),
    ];
    ctl.clono_print_opt.cvars = vec!["u".to_string(), "const".to_string(), "notes".to_string()];
    ctl.clono_print_opt.lvars = vec!["datasets".to_string(), "n".to_string()];

    ctl.clono_group_opt.min_group = 1;

    ctl.allele_alg_opt.min_mult = 4;
    ctl.allele_alg_opt.min_alt = 4;

    ctl.join_alg_opt.max_score = 500_000.0;
    ctl.join_alg_opt.merge_onesies = true; // should just kill this as an option
    ctl.join_alg_opt.merge_onesies_ctl = true;
    ctl.join_alg_opt.max_cdr3_diffs = 15;

    ctl.join_print_opt.pfreq = 1_000_000_000;
    ctl.join_print_opt.quiet = true;

    ctl.parseable_opt.pchains = "4".to_string();

    // Pretest for consistency amongst TCR, BCR, GEX and META.  Also preparse GEX.

    let (mut have_tcr, mut have_bcr) = (false, false);
    let mut have_gex = false;
    let mut have_meta = false;
    let mut gex = String::new();
    let mut bc = String::new();
    let mut metas = Vec::<String>::new();
    let mut metaxs = Vec::<String>::new();
    let mut xcrs = Vec::<String>::new();
    for i in 1..args.len() {
        if args[i].starts_with("BI=") {
            have_bcr = true;
            have_gex = true;
        } else if args[i].starts_with("TCR=") {
            have_tcr = true;
        } else if args[i].starts_with("BCR=") {
            have_bcr = true;
        } else if args[i].starts_with("GEX=") {
            have_gex = true;
        } else if args[i].starts_with("META=") || args[i].starts_with("METAX=") {
            have_meta = true;
        }
        if args[i].starts_with("GEX=") {
            gex = args[i].after("GEX=").to_string();
        }
        if args[i].starts_with("BC=") {
            bc = args[i].after("BC=").to_string();
        }
        if is_simple_arg(&args[i], "MARK_STATS")? {
            ctl.gen_opt.mark_stats = true;
        }
        if is_simple_arg(&args[i], "MARK_STATS2")? {
            ctl.gen_opt.mark_stats2 = true;
        }
        if is_simple_arg(&args[i], "MARKED_B")? {
            ctl.clono_filt_opt_def.marked_b = true;
        }
    }
    if have_meta && (have_tcr || have_bcr || have_gex || !bc.is_empty()) {
        return Err(
            "\nIf META is specified, then none of TCR, BCR, GEX or BC can be specified.\n"
                .to_string(),
        );
    }
    if have_tcr && have_bcr {
        return Err("\nKindly please do not specify both TCR and BCR.\n".to_string());
    }
    let mut using_plot = false;

    // Preprocess BI argument.

    for i in 1..args.len() {
        if args[i].starts_with("BI=") {
            if !ctl.gen_opt.internal_run {
                return Err(format!("\nUnrecognized argument {}.\n", args[i]));
            }
            let x = args[i].after("BI=").split(',').collect::<Vec<&str>>();
            let mut y = Vec::<String>::new();
            for j in 0..x.len() {
                if x[j].contains('-') {
                    let (start, stop) = (x[j].before("-"), x[j].after("-"));
                    if start.parse::<usize>().is_err()
                        || stop.parse::<usize>().is_err()
                        || start.force_usize() > stop.force_usize()
                    {
                        return Err("\nIllegal range in BI argument.\n".to_string());
                    }
                    let (start, stop) = (start.force_usize(), stop.force_usize());
                    for j in start..=stop {
                        y.push(format!("{}", j));
                    }
                } else {
                    y.push(x[j].to_string());
                }
            }
            let mut args2 = Vec::<String>::new();
            for j in 0..i {
                args2.push(args[j].clone());
            }
            let f = include_str!["../../enclone/src/enclone.testdata.bcr.gex"];
            let (mut bcrv, mut gexv) = (Vec::<String>::new(), Vec::<String>::new());
            for n in y.iter() {
                if *n != "m1" {
                    if n.parse::<usize>().is_err() || n.force_usize() < 1 || n.force_usize() > 12 {
                        return Err(
                            "\nBI only works for values n with if 1 <= n <= 12, or n = m1.\n"
                                .to_string(),
                        );
                    }
                } else if y.len() > 1 {
                    return Err(
                        "\nFor BI, if you specify m1, you can only specify m1.\n".to_string()
                    );
                }
                let mut found = false;
                for s in f.lines() {
                    if s == format!("DONOR={}", n) {
                        found = true;
                    } else if found && s.starts_with("DONOR=") {
                        break;
                    }
                    if found {
                        if s.starts_with("BCR=") {
                            bcrv.push(s.after("BCR=").to_string());
                        }
                        if s.starts_with("GEX=") {
                            gexv.push(s.after("GEX=").to_string());
                        }
                        if s == "SPECIES=mouse" {
                            args2.push("MOUSE".to_string());
                        }
                    }
                }
            }
            args2.push(format!("BCR={}", bcrv.iter().format(";")));
            args2.push(format!("GEX={}", gexv.iter().format(";")));
            gex = format!("{}", gexv.iter().format(";"));
            for j in i + 1..args.len() {
                args2.push(args[j].clone());
            }
            args = args2;
            break;
        }
    }

    // Preprocess NALL and NALL_GEX.

    for i in 1..args.len() {
        if args[i] == *"NALL" || args[i] == "NALL_CELL" || args[i] == "NALL_GEX" {
            let f = [
                "NCELL",
                "NGEX",
                "NCROSS",
                "NDOUBLET",
                "NUMI",
                "NUMI_RATIO",
                "NGRAPH_FILTER",
                "NQUAL",
                "NWEAK_CHAINS",
                "NWEAK_ONESIES",
                "NFOURSIE_KILL",
                "NWHITEF",
                "NBC_DUP",
                "MIX_DONORS",
                "NIMPROPER",
            ];
            for j in 0..f.len() {
                if f[j] == "NCELL" {
                    if args[i] != "NALL_CELL" {
                        args.push(f[j].to_string());
                    }
                } else if f[j] == "NGEX" {
                    if args[i] != "NALL_GEX" {
                        args.push(f[j].to_string());
                    }
                } else {
                    args.push(f[j].to_string());
                }
            }
            break;
        }
    }

    // Define arguments that set something to true.

    let mut set_true = vec![
        ("ACCEPT_BROKEN", &mut ctl.gen_opt.accept_broken),
        ("ACCEPT_INCONSISTENT", &mut ctl.gen_opt.accept_inconsistent),
        ("ACCEPT_REUSE", &mut ctl.gen_opt.accept_reuse),
        (
            "ALIGN_JALIGN_CONSISTENCY",
            &mut ctl.gen_opt.align_jun_align_consistency,
        ),
        ("ALLOW_INCONSISTENT", &mut ctl.gen_opt.allow_inconsistent),
        ("ANN", &mut ctl.join_print_opt.ann),
        ("ANN0", &mut ctl.join_print_opt.ann0),
        ("BARCODES", &mut ctl.clono_print_opt.barcodes),
        ("BASELINE", &mut ctl.gen_opt.baseline),
        ("BCJOIN", &mut ctl.join_alg_opt.bcjoin),
        ("BUILT_IN", &mut ctl.gen_opt.built_in),
        ("CDIFF", &mut ctl.clono_filt_opt.cdiff),
        ("CHAIN_BRIEF", &mut ctl.clono_print_opt.chain_brief),
        ("COMPLETE", &mut ctl.gen_opt.complete),
        ("CON", &mut ctl.allele_print_opt.con),
        ("CON_CON", &mut ctl.gen_opt.con_con),
        ("CON_TRACE", &mut ctl.allele_print_opt.con_trace),
        ("CONP", &mut ctl.clono_print_opt.conp),
        ("CONX", &mut ctl.clono_print_opt.conx),
        ("CURRENT_REF", &mut ctl.gen_opt.current_ref),
        ("DEBUG_TABLE_PRINTING", &mut ctl.debug_table_printing),
        ("DEL", &mut ctl.clono_filt_opt.del),
        ("DESCRIP", &mut ctl.gen_opt.descrip),
        ("D_INCONSISTENT", &mut ctl.clono_filt_opt.d_inconsistent),
        ("D_NONE", &mut ctl.clono_filt_opt.d_none),
        ("D_SECOND", &mut ctl.clono_filt_opt.d_second),
        ("EASY", &mut ctl.join_alg_opt.easy),
        ("ECHO", &mut ctl.gen_opt.echo),
        ("FOLD_HEADERS", &mut ctl.gen_opt.fold_headers),
        ("FORCE", &mut ctl.force),
        ("FULL_SEQC", &mut ctl.clono_print_opt.full_seqc),
        ("GRAPH", &mut ctl.gen_opt.graph),
        ("HAVE_ONESIE", &mut ctl.clono_filt_opt.have_onesie),
        ("HEAVY_CHAIN_REUSE", &mut ctl.gen_opt.heavy_chain_reuse),
        ("IMGT", &mut ctl.gen_opt.imgt),
        ("IMGT_FIX", &mut ctl.gen_opt.imgt_fix),
        ("INDELS", &mut ctl.gen_opt.indels),
        ("INFO_RESOLVE", &mut ctl.gen_opt.info_resolve),
        ("INKT", &mut ctl.clono_filt_opt.inkt),
        ("INSERTIONS", &mut ctl.gen_opt.insertions),
        ("INTERNAL", &mut ctl.gen_opt.internal_run),
        ("JC1", &mut ctl.gen_opt.jc1),
        ("MAIT", &mut ctl.clono_filt_opt.mait),
        ("MARKED", &mut ctl.clono_filt_opt.marked),
        ("MEAN", &mut ctl.clono_print_opt.mean),
        ("MIX_DONORS", &mut ctl.clono_filt_opt_def.donor),
        ("MOUSE", &mut ctl.gen_opt.mouse),
        ("NCELL", &mut ctl.gen_opt.ncell),
        ("NCROSS", &mut ctl.clono_filt_opt_def.ncross),
        ("NEWICK", &mut ctl.gen_opt.newick),
        ("NGEX", &mut ctl.clono_filt_opt_def.ngex),
        ("NGRAPH_FILTER", &mut ctl.gen_opt.ngraph_filter),
        ("NGROUP", &mut ctl.clono_group_opt.ngroup),
        ("NIMPROPER", &mut ctl.merge_all_impropers),
        ("NO_NEWLINE", &mut ctl.gen_opt.no_newline),
        ("NO_UNCAP_SIM", &mut ctl.gen_opt.no_uncap_sim),
        ("NON_CELL_MARK", &mut ctl.clono_filt_opt_def.non_cell_mark),
        ("NOPRINT", &mut ctl.gen_opt.noprint),
        ("NOPRINTX", &mut ctl.gen_opt.noprintx),
        ("NOTE_SIMPLE", &mut ctl.clono_print_opt.note_simple),
        ("NPLAIN", &mut ctl.pretty),
        ("NWHITEF", &mut ctl.gen_opt.nwhitef),
        ("NWARN", &mut ctl.gen_opt.nwarn),
        ("PCELL", &mut ctl.parseable_opt.pbarcode),
        ("PG_READABLE", &mut ctl.gen_opt.peer_group_readable),
        ("PER_CELL", &mut ctl.clono_print_opt.bu),
        ("PROTECT_BADS", &mut ctl.clono_filt_opt.protect_bads),
        ("QUAD_HIVE", &mut ctl.plot_opt.plot_quad),
        ("RE", &mut ctl.gen_opt.reannotate),
        ("REPROD", &mut ctl.gen_opt.reprod),
        ("REQUIRE_UNBROKEN_OK", &mut ctl.gen_opt.require_unbroken_ok),
        ("REUSE", &mut ctl.gen_opt.reuse),
        ("ROW_FILL_VERBOSE", &mut ctl.gen_opt.row_fill_verbose),
        ("SCAN_EXACT", &mut ctl.gen_opt.gene_scan_exact),
        ("SEQC", &mut ctl.clono_print_opt.seqc),
        ("SHOW_BC", &mut ctl.join_print_opt.show_bc),
        ("STABLE_DOC", &mut ctl.gen_opt.stable_doc),
        (
            "SPLIT_PLOT_BY_ORIGIN",
            &mut ctl.plot_opt.split_plot_by_origin,
        ),
        ("SUM", &mut ctl.clono_print_opt.sum),
        ("SUMMARY", &mut ctl.gen_opt.summary),
        ("SUMMARY_CLEAN", &mut ctl.gen_opt.summary_clean),
        ("SUMMARY_CSV", &mut ctl.gen_opt.summary_csv),
        (
            "SUPPRESS_ISOTYPE_LEGEND",
            &mut ctl.plot_opt.plot_by_isotype_nolegend,
        ),
        ("TOP_GENES", &mut ctl.gen_opt.top_genes),
        ("TOY", &mut ctl.gen_opt.toy),
        ("TOY_COM", &mut ctl.gen_opt.toy_com),
        ("UMI_FILT_MARK", &mut ctl.clono_filt_opt_def.umi_filt_mark),
        (
            "UMI_RATIO_FILT_MARK",
            &mut ctl.clono_filt_opt_def.umi_ratio_filt_mark,
        ),
        ("UNACCOUNTED", &mut ctl.perf_opt.unaccounted),
        ("UTR_CON", &mut ctl.gen_opt.utr_con),
        ("VDUP", &mut ctl.clono_filt_opt.vdup),
        ("WEAK", &mut ctl.gen_opt.weak),
        ("WHITEF", &mut ctl.clono_filt_opt_def.whitef),
    ];

    // Define arguments that set something to false.

    let mut set_false = vec![
        ("H5_SLICE", &mut ctl.gen_opt.h5_pre),
        ("NBC_DUP", &mut ctl.clono_filt_opt_def.bc_dup),
        ("NDOUBLET", &mut ctl.clono_filt_opt_def.doublet),
        ("NFOURSIE_KILL", &mut ctl.clono_filt_opt_def.weak_foursies),
        ("NMERGE_ONESIES", &mut ctl.join_alg_opt.merge_onesies_ctl),
        ("NQUAL", &mut ctl.clono_filt_opt.qual_filter),
        ("NSIG", &mut ctl.clono_filt_opt_def.signature),
        ("NSILENT", &mut ctl.silent),
        ("NUMI", &mut ctl.clono_filt_opt_def.umi_filt),
        ("NUMI_RATIO", &mut ctl.clono_filt_opt_def.umi_ratio_filt),
        ("NWEAK_CHAINS", &mut ctl.clono_filt_opt_def.weak_chains),
        ("NWEAK_ONESIES", &mut ctl.clono_filt_opt_def.weak_onesies),
        ("PRINT_FAILED_JOINS", &mut ctl.join_print_opt.quiet),
    ];

    // Define arguments that set something to a usize.

    let set_usize = [
        ("CHAINS_EXACT", &mut ctl.gen_opt.chains_exact),
        ("MAX_CDR3_DIFFS", &mut ctl.join_alg_opt.max_cdr3_diffs),
        ("MAX_DATASETS", &mut ctl.clono_filt_opt.max_datasets),
        ("MAX_DEGRADATION", &mut ctl.heur.max_degradation),
        ("MAX_DIFFS", &mut ctl.heur.max_diffs),
        ("MAX_EXACTS", &mut ctl.clono_filt_opt.max_exacts),
        ("MIN_ALT", &mut ctl.allele_alg_opt.min_alt),
        ("MIN_CELLS_EXACT", &mut ctl.gen_opt.min_cells_exact),
        ("MIN_CHAINS_EXACT", &mut ctl.gen_opt.min_chains_exact),
        (
            "MIN_DATASET_RATIO",
            &mut ctl.clono_filt_opt.min_dataset_ratio,
        ),
        ("MIN_DATASETS", &mut ctl.clono_filt_opt.min_datasets),
        ("MIN_EXACTS", &mut ctl.clono_filt_opt.min_exacts),
        ("MIN_GROUP", &mut ctl.clono_group_opt.min_group),
        ("MIN_MULT", &mut ctl.allele_alg_opt.min_mult),
        ("MIN_UMIS", &mut ctl.clono_filt_opt.min_umi),
        ("PFREQ", &mut ctl.join_print_opt.pfreq),
    ];

    // Define arguments that set something to an i32.

    let set_i32 = [
        ("JSCORE_GAP_EXTEND", &mut ctl.gen_opt.jscore_gap_extend),
        ("JSCORE_GAP_OPEN", &mut ctl.gen_opt.jscore_gap_open),
        ("JSCORE_MATCH", &mut ctl.gen_opt.jscore_match),
        ("JSCORE_MISMATCH", &mut ctl.gen_opt.jscore_mismatch),
    ];

    // Define arguments that set something to an f64.

    let set_f64 = [("JSCORE_BITS_MULT", &mut ctl.gen_opt.jscore_bits_multiplier)];

    // Define arguments that set something to a string.

    let set_string = [
        ("AG_CENTER", &mut ctl.clono_group_opt.asymmetric_center),
        (
            "AG_DIST_BOUND",
            &mut ctl.clono_group_opt.asymmetric_dist_bound,
        ),
        (
            "AG_DIST_FORMULA",
            &mut ctl.clono_group_opt.asymmetric_dist_formula,
        ),
        ("CLUSTAL_AA", &mut ctl.gen_opt.clustal_aa),
        ("CLUSTAL_DNA", &mut ctl.gen_opt.clustal_dna),
        ("CONFIG", &mut ctl.gen_opt.config_file),
        ("EXT", &mut ctl.gen_opt.ext),
        ("PCHAINS", &mut ctl.parseable_opt.pchains),
        ("POUT", &mut ctl.parseable_opt.pout),
        ("TRACE_BARCODE", &mut ctl.gen_opt.trace_barcode),
    ];

    // Define arguments that set something to a string that is an output file name.

    let set_string_writeable = [
        ("BINARY", &mut ctl.gen_opt.binary),
        ("DONOR_REF_FILE", &mut ctl.gen_opt.dref_file),
        ("HONEY_OUT", &mut ctl.plot_opt.honey_out),
        ("PROTO", &mut ctl.gen_opt.proto),
        ("SUBSET_JSON", &mut ctl.gen_opt.subset_json),
    ];

    // Define arguments that set something to a string that is an output file name or stdout.

    let set_string_writeable_or_stdout = [
        ("PEER_GROUP", &mut ctl.gen_opt.peer_group_filename),
        ("PHYLIP_AA", &mut ctl.gen_opt.phylip_aa),
        ("PHYLIP_DNA", &mut ctl.gen_opt.phylip_dna),
    ];

    // Define arguments that set something to a string that is an input file name, represented
    // as an option.

    let set_string_readable = [
        (
            "CLONOTYPE_GROUP_NAMES",
            &mut ctl.gen_opt.clonotype_group_names,
        ),
        ("HONEY_IN", &mut ctl.plot_opt.honey_in),
        ("PROTO_METADATA", &mut ctl.gen_opt.proto_metadata),
    ];

    // Define arguments that set something to a string that is an input file name, not represented
    // as an option.

    let set_string_readable_plain = [("REF", &mut ctl.gen_opt.refname)];

    // Define arguments that do nothing (because already parsed), and which have no "= value" part.

    let set_nothing_simple = [
        "CELLRANGER",
        "COMP",
        "COMPE",
        "COMP2",
        "CTRLC",
        "DUMP_INTERNAL_IDS",
        "EVIL_EYE",
        "FORCE_EXTERNAL",
        "LONG_HELP",
        "MARKED_B",
        "MARK_STATS",
        "MARK_STATS2",
        "NALL",
        "NALL_CELL",
        "NALL_GEX",
        "NO_KILL",
        "NOPAGER",
        "NOPRETTY",
        "PLAIN",
        "PRINT_CPU",
        "PRINT_CPU_INFO",
        "PROFILE",
        "SVG",
    ];

    // Define arguments that do nothing (because already parsed), and which may have
    // an "= value" part.

    let set_nothing = [
        "BC",
        "BI",
        "EMAIL",
        "GEX",
        "HTML",
        "INTERNAL",
        "BUG_REPORTS",
        "PRE",
        "SOURCE",
        "VERBOSE",
    ];

    // Define arguments that set something to a string that is an input CSV file name.

    let set_string_readable_csv = [("INFO", &mut ctl.gen_opt.info)];

    // Traverse arguments.

    let mut processed = vec![true; args.len()];
    if ctl.gen_opt.evil_eye {
        println!("starting main args loop");
    }
    'args_loop: for i in 1..args.len() {
        let mut arg = args[i].to_string();
        if ctl.gen_opt.evil_eye {
            println!("processing arg = {}", arg);
        }

        // Replace deprecated option.

        if arg == *"KEEP_IMPROPER" {
            arg = "NIMPROPER".to_string();
        }

        // Strip out certain quoted expressions.

        if arg.contains("=\"") && arg.ends_with('\"') {
            let mut quotes = 0;
            for c in arg.chars() {
                if c == '\"' {
                    quotes += 1;
                }
            }
            if quotes == 2 {
                arg = format!("{}={}", arg.before("="), arg.between("\"", "\""));
            }
        }
        args[i] = arg.clone();
        let arg = arg;

        // Check for weird case that might arise if testing code is screwed up.

        if arg.is_empty() {
            return Err(
                "\nYou've passed a null argument to enclone.  Normally that isn't \
                 possible.\nPlease take a detailed look at how you're invoking enclone.\n"
                    .to_string(),
            );
        }

        // Process set_true arguments.

        for j in 0..set_true.len() {
            if arg == *set_true[j].0 {
                *(set_true[j].1) = true;
                continue 'args_loop;
            }
        }

        // Process set_false arguments.

        for j in 0..set_false.len() {
            if arg == *set_false[j].0 {
                *(set_false[j].1) = false;
                continue 'args_loop;
            }
        }

        // Process set_usize args.

        for j in 0..set_usize.len() {
            if is_usize_arg(&arg, set_usize[j].0)? {
                *(set_usize[j].1) = arg.after(&format!("{}=", set_usize[j].0)).force_usize();
                continue 'args_loop;
            }
        }

        // Process set_i32 args.

        for j in 0..set_i32.len() {
            if is_i32_arg(&arg, set_i32[j].0)? {
                *(set_i32[j].1) = arg.after(&format!("{}=", set_i32[j].0)).force_i32();
                continue 'args_loop;
            }
        }

        // Process set_f64 args.

        for j in 0..set_f64.len() {
            if is_f64_arg(&arg, set_f64[j].0)? {
                *(set_f64[j].1) = arg.after(&format!("{}=", set_f64[j].0)).force_f64();
                continue 'args_loop;
            }
        }

        // Process set_string args.

        for j in 0..set_string.len() {
            if is_string_arg(&arg, set_string[j].0)? {
                *(set_string[j].1) = arg.after(&format!("{}=", set_string[j].0)).to_string();
                continue 'args_loop;
            }
        }

        // Process set_string_writeable args.

        for j in 0..set_string_writeable.len() {
            let var = &set_string_writeable[j].0;
            if is_string_arg(&arg, var)? {
                *(set_string_writeable[j].1) = arg.after(&format!("{}=", var)).to_string();
                *(set_string_writeable[j].1) =
                    stringme(&tilde_expand(set_string_writeable[j].1.as_bytes()));
                let val = &(set_string_writeable[j].1);
                if ctl.gen_opt.evil_eye {
                    println!("creating file {} to test writability", val);
                }
                let f = File::create(&val);
                if f.is_err() {
                    let mut msgx = format!(
                        "\nYou've specified an output file\n{}\nthat cannot be written.\n",
                        val
                    );
                    if val.contains('/') {
                        let dir = val.rev_before("/");
                        let msg;
                        if path_exists(dir) {
                            msg = "exists";
                        } else {
                            msg = "does not exist";
                        }
                        msgx += &mut format!("Note that the path {} {}.\n", dir, msg);
                    }
                    return Err(msgx);
                }
                if ctl.gen_opt.evil_eye {
                    println!("removing file {}", val);
                }
                remove_file(&val).unwrap_or_else(|_| panic!("could not remove file {}", val));
                if ctl.gen_opt.evil_eye {
                    println!("removal of file {} complete", val);
                }
                continue 'args_loop;
            }
        }

        // Process set_string_writeable_or_stdout args.

        for j in 0..set_string_writeable_or_stdout.len() {
            let var = &set_string_writeable_or_stdout[j].0;
            if is_string_arg(&arg, var)? {
                *(set_string_writeable_or_stdout[j].1) =
                    arg.after(&format!("{}=", var)).to_string();
                *(set_string_writeable_or_stdout[j].1) = stringme(&tilde_expand(
                    set_string_writeable_or_stdout[j].1.as_bytes(),
                ));
                let val = &(set_string_writeable_or_stdout[j].1);
                if *val != "stdout" {
                    if ctl.gen_opt.evil_eye {
                        println!("creating file {} to test writability, not stdout", val);
                    }
                    let f = File::create(&val);
                    if f.is_err() {
                        let mut msgx = format!(
                            "\nYou've specified an output file\n{}\nthat cannot be written.\n",
                            val
                        );
                        if val.contains('/') {
                            let dir = val.rev_before("/");
                            let msg;
                            if path_exists(dir) {
                                msg = "exists";
                            } else {
                                msg = "does not exist";
                            }
                            msgx += &mut format!("Note that the path {} {}.\n", dir, msg);
                        }
                        return Err(msgx);
                    }
                    if ctl.gen_opt.evil_eye {
                        println!("removing file {}", val);
                    }
                    remove_file(&val).unwrap_or_else(|_| panic!("could not remove file {}", val));
                }
                if ctl.gen_opt.evil_eye {
                    println!("removal of file {} complete", val);
                }
                continue 'args_loop;
            }
        }

        // Process set_string_readable args.

        for j in 0..set_string_readable.len() {
            let var = &set_string_readable[j].0;
            if is_string_arg(&arg, var)? {
                let mut val = arg.after(&format!("{}=", var)).to_string();
                if val.is_empty() {
                    return Err(format!("\nFilename input in {} cannot be empty.\n", val));
                }
                val = stringme(&tilde_expand(val.as_bytes()));
                *(set_string_readable[j].1) = Some(val.clone());
                if ctl.gen_opt.evil_eye {
                    println!("testing ability to open file {}", val);
                }
                require_readable_file(&val, &arg)?;
                if ctl.gen_opt.evil_eye {
                    println!("file open complete");
                }
                continue 'args_loop;
            }
        }

        // Process set_string_readable_plain args.

        for j in 0..set_string_readable_plain.len() {
            let var = &set_string_readable_plain[j].0;
            if is_string_arg(&arg, var)? {
                let mut val = arg.after(&format!("{}=", var)).to_string();
                if val.is_empty() {
                    return Err(format!("\nFilename input in {} cannot be empty.\n", val));
                }
                val = stringme(&tilde_expand(val.as_bytes()));
                *(set_string_readable_plain[j].1) = val.clone();
                if ctl.gen_opt.evil_eye {
                    println!("testing ability to open file {}", val);
                }
                require_readable_file(&val, &arg)?;
                if ctl.gen_opt.evil_eye {
                    println!("file open complete");
                }
                continue 'args_loop;
            }
        }

        // Process set_string_readable_csv args.

        for j in 0..set_string_readable_csv.len() {
            let var = &set_string_readable_csv[j].0;
            if is_string_arg(&arg, var)? {
                let mut val = arg.after(&format!("{}=", var)).to_string();
                if val.is_empty() {
                    return Err(format!("\nFilename input in {} cannot be empty.\n", val));
                }
                val = stringme(&tilde_expand(val.as_bytes()));
                if !val.ends_with(".csv") {
                    return Err(format!(
                        "\nFilename input in {} needs to end with .csv.\n",
                        val
                    ));
                }
                *(set_string_readable_csv[j].1) = Some(val.clone());
                require_readable_file(&val, &arg)?;
                continue 'args_loop;
            }
        }

        // Process set_nothing_simple args.

        for j in 0..set_nothing_simple.len() {
            if arg == *set_nothing_simple[j] {
                continue 'args_loop;
            }
        }

        // Process set_nothing args.

        for j in 0..set_nothing.len() {
            if arg == *set_nothing[j] || arg.starts_with(&format!("{}=", set_nothing[j])) {
                continue 'args_loop;
            }
        }

        // Otherwise mark as not processed.

        processed[i] = false;
    }

    // Process remaining args.

    if ctl.gen_opt.evil_eye {
        println!("processing remaining args");
    }
    for i in 1..args.len() {
        if processed[i] {
            continue;
        }
        process_special_arg(
            &args[i],
            &mut ctl,
            &mut metas,
            &mut metaxs,
            &mut xcrs,
            &mut using_plot,
        )?;
    }

    // Record time.

    ctl.perf_stats(&targs, "in main args loop");

    // Do residual argument processing.

    if ctl.gen_opt.internal_run && ctl.gen_opt.config.is_empty() {
        return Err(
            "\nYou need to set up your configuration file, please ask for help.\n".to_string(),
        );
    }
    proc_args_post(
        &mut ctl, &args, &metas, &metaxs, &xcrs, have_gex, &gex, &bc, using_plot,
    )?;
    Ok(())
}
