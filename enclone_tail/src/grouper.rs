// Copyright (c) 2021 10X Genomics, Inc. All rights reserved.

// This is the actual grouping code.
//
// output: a vector, each element of which is a group object
//
// group object: a vector of pairs (i, msg) where i is an index into exacts and msg is a message
//               to be printed

use amino::*;
use edit_distance::edit_distance;
use enclone_core::defs::*;
use equiv::EquivRel;
use rayon::prelude::*;
use string_utils::*;
use vdj_ann::refx::*;
use vector_utils::*;

pub fn grouper(
    refdata: &RefData,
    exacts: &Vec<Vec<usize>>,
    in_center: &Vec<bool>,
    exact_clonotypes: &Vec<ExactClonotype>,
    ctl: &EncloneControl,
    rsi: &Vec<ColInfo>,
    opt_d_val: &Vec<(usize, Vec<Vec<Vec<usize>>>)>,
) -> Vec<Vec<(i32, String)>> {
    // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

    // Case 0: no grouping.

    if ctl.clono_group_opt.style.is_empty() {
        let mut groups = Vec::<Vec<(i32, String)>>::new();
        let mut grepsn = Vec::<usize>::new();
        for i in 0..exacts.len() {
            groups.push(vec![(i as i32, String::new())]);
            let mut cells = 0;
            for u in exacts[i].iter() {
                cells += exact_clonotypes[*u].ncells();
            }
            grepsn.push(cells);
        }
        sort_sync2(&mut grepsn, &mut groups);
        groups.reverse();
        groups

    // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

    // Case 1: symmetric grouping.
    } else if ctl.clono_group_opt.style == "symmetric" {
        let mut e: EquivRel = EquivRel::new(exacts.len() as i32);
        let mut group = Vec::<usize>::new();
        for i in 0..exacts.len() {
            group.push(i);
        }
        let mut groups = vec![group];

        // Group by vj_refname.

        if ctl.clono_group_opt.vj_refname {
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut all = Vec::new();
                for i in g.iter() {
                    let mut s = Vec::<String>::new();
                    for col in 0..rsi[*i].mat.len() {
                        let vid = rsi[*i].vids[col];
                        let jid = rsi[*i].jids[col];
                        s.push(refdata.name[vid].clone());
                        s.push(refdata.name[jid].clone());
                    }
                    all.push((s, *i));
                }
                all.sort();
                let mut i = 0;
                while i < all.len() {
                    let j = next_diff1_2(&all, i as i32) as usize;
                    let mut g = Vec::<usize>::new();
                    for k in i..j {
                        g.push(all[k].1);
                    }
                    groups2.push(g);
                    i = j;
                }
            }
            groups = groups2;
        }

        // Group by vdj_refname.

        if ctl.clono_group_opt.vdj_refname {
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut all = Vec::new();
                for i in g.iter() {
                    let mut s = Vec::<String>::new();
                    for col in 0..rsi[*i].mat.len() {
                        let vid = rsi[*i].vids[col];
                        let jid = rsi[*i].jids[col];
                        s.push(refdata.name[vid].clone());
                        if rsi[*i].left[col] {
                            let d = &opt_d_val[*i].1[col][0];
                            let dname;
                            if !d.is_empty() {
                                dname = refdata.name[d[0]].clone();
                            } else {
                                dname = "null".to_string();
                            }
                            s.push(dname);
                        }
                        s.push(refdata.name[jid].clone());
                    }
                    all.push((s, *i));
                }
                all.sort();
                let mut i = 0;
                while i < all.len() {
                    let j = next_diff1_2(&all, i as i32) as usize;
                    let mut g = Vec::<usize>::new();
                    for k in i..j {
                        g.push(all[k].1);
                    }
                    groups2.push(g);
                    i = j;
                }
            }
            groups = groups2;
        }

        // Group by vj_heavy_refname.

        if ctl.clono_group_opt.vj_heavy_refname {
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut all = Vec::new();
                for i in g.iter() {
                    let mut s = Vec::<String>::new();
                    for col in 0..rsi[*i].mat.len() {
                        if rsi[*i].left[col] {
                            let vid = rsi[*i].vids[col];
                            let jid = rsi[*i].jids[col];
                            s.push(refdata.name[vid].clone());
                            s.push(refdata.name[jid].clone());
                        }
                    }
                    all.push((s, *i));
                }
                all.sort();
                let mut i = 0;
                while i < all.len() {
                    let j = next_diff1_2(&all, i as i32) as usize;
                    if all[i].0.is_empty() {
                        for k in i..j {
                            let g = vec![all[k].1];
                            groups2.push(g);
                        }
                    } else {
                        let mut g = Vec::<usize>::new();
                        for k in i..j {
                            g.push(all[k].1);
                        }
                        groups2.push(g);
                    }
                    i = j;
                }
            }
            groups = groups2;
        }

        // Group by vdj_heavy_refname.

        if ctl.clono_group_opt.vdj_heavy_refname {
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut all = Vec::new();
                for i in g.iter() {
                    let mut s = Vec::<String>::new();
                    for col in 0..rsi[*i].mat.len() {
                        if rsi[*i].left[col] {
                            let vid = rsi[*i].vids[col];
                            let jid = rsi[*i].jids[col];
                            s.push(refdata.name[vid].clone());
                            let d = &opt_d_val[*i].1[col][0];
                            let dname;
                            if !d.is_empty() {
                                dname = refdata.name[d[0]].clone();
                            } else {
                                dname = "null".to_string();
                            }
                            s.push(dname);
                            s.push(refdata.name[jid].clone());
                        }
                    }
                    all.push((s, *i));
                }
                all.sort();
                let mut i = 0;
                while i < all.len() {
                    let j = next_diff1_2(&all, i as i32) as usize;
                    if all[i].0.is_empty() {
                        for k in i..j {
                            let g = vec![all[k].1];
                            groups2.push(g);
                        }
                    } else {
                        let mut g = Vec::<usize>::new();
                        for k in i..j {
                            g.push(all[k].1);
                        }
                        groups2.push(g);
                    }
                    i = j;
                }
            }
            groups = groups2;
        }

        // Group by vj_len.

        if ctl.clono_group_opt.vj_len {
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut all = Vec::new();
                for i in g.iter() {
                    let ex = &exact_clonotypes[exacts[*i][0]];
                    let mut s = Vec::<(bool, usize)>::new();
                    for j in 0..ex.share.len() {
                        s.push((ex.share[j].left, ex.share[j].seq_del.len()));
                    }
                    s.sort_unstable();
                    all.push((s, *i));
                }
                all.sort();
                let mut i = 0;
                while i < all.len() {
                    let j = next_diff1_2(&all, i as i32) as usize;
                    let mut g = Vec::<usize>::new();
                    for k in i..j {
                        g.push(all[k].1);
                    }
                    groups2.push(g);
                    i = j;
                }
            }
            groups = groups2;
        }

        // Group by cdr3_len.

        if ctl.clono_group_opt.cdr3_len {
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut all = Vec::new();
                for i in g.iter() {
                    let ex = &exact_clonotypes[exacts[*i][0]];
                    let mut s = Vec::<(bool, usize)>::new();
                    for j in 0..ex.share.len() {
                        s.push((ex.share[j].left, ex.share[j].cdr3_aa.len()));
                    }
                    s.sort_unstable();
                    all.push((s, *i));
                }
                all.sort();
                let mut i = 0;
                while i < all.len() {
                    let j = next_diff1_2(&all, i as i32) as usize;
                    let mut g = Vec::<usize>::new();
                    for k in i..j {
                        g.push(all[k].1);
                    }
                    groups2.push(g);
                    i = j;
                }
            }
            groups = groups2;
        }

        // Group by aa_heavy_pc.

        if ctl.clono_group_opt.aa_heavy_pc.is_some() {
            let min_r = ctl.clono_group_opt.aa_heavy_pc.unwrap() / 100.0;
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut ee: EquivRel = EquivRel::new(g.len() as i32);
                for i1 in 0..g.len() {
                    'next_heavy: for i2 in i1 + 1..g.len() {
                        if ee.class_id(i1 as i32) == ee.class_id(i2 as i32) {
                            continue;
                        }
                        let (g1, g2) = (g[i1], g[i2]);
                        for u1 in exacts[g1].iter() {
                            for u2 in exacts[g2].iter() {
                                let (ex1, ex2) = (&exact_clonotypes[*u1], &exact_clonotypes[*u2]);
                                for p1 in 0..ex1.share.len() {
                                    if !ex1.share[p1].left {
                                        continue;
                                    }
                                    let dna1 = &ex1.share[p1].seq;
                                    for p2 in 0..ex2.share.len() {
                                        if !ex2.share[p2].left {
                                            continue;
                                        }
                                        let dna2 = &ex2.share[p2].seq;
                                        let (aa1, aa2) = (aa_seq(dna1, 0), aa_seq(dna2, 0));
                                        let d = edit_distance(strme(&aa1), strme(&aa2));
                                        let r1 = if d <= aa1.len() { aa1.len() - d } else { 0 };
                                        let r1 = r1 as f64 / aa1.len() as f64;
                                        let r2 = if d <= aa2.len() { aa2.len() - d } else { 0 };
                                        let r2 = r2 as f64 / aa2.len() as f64;
                                        if r1 >= min_r || r2 >= min_r {
                                            ee.join(i1 as i32, i2 as i32);
                                            continue 'next_heavy;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let mut reps = Vec::<i32>::new();
                ee.orbit_reps(&mut reps);
                for i in 0..reps.len() {
                    let mut o = Vec::<i32>::new();
                    ee.orbit(i as i32, &mut o);
                    let mut p = Vec::<usize>::new();
                    for j in 0..o.len() {
                        p.push(g[o[j] as usize]);
                    }
                    groups2.push(p);
                }
            }
            groups = groups2;
        }

        // Group by aa_light_pc.

        if ctl.clono_group_opt.aa_light_pc.is_some() {
            let min_r = ctl.clono_group_opt.aa_light_pc.unwrap() / 100.0;
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut ee: EquivRel = EquivRel::new(g.len() as i32);
                for i1 in 0..g.len() {
                    'next_light: for i2 in i1 + 1..g.len() {
                        if ee.class_id(i1 as i32) == ee.class_id(i2 as i32) {
                            continue;
                        }
                        let (g1, g2) = (g[i1], g[i2]);
                        for u1 in exacts[g1].iter() {
                            for u2 in exacts[g2].iter() {
                                let (ex1, ex2) = (&exact_clonotypes[*u1], &exact_clonotypes[*u2]);
                                for p1 in 0..ex1.share.len() {
                                    if ex1.share[p1].left {
                                        continue;
                                    }
                                    let dna1 = &ex1.share[p1].seq;
                                    for p2 in 0..ex2.share.len() {
                                        if ex2.share[p2].left {
                                            continue;
                                        }
                                        let dna2 = &ex2.share[p2].seq;
                                        let (aa1, aa2) = (aa_seq(dna1, 0), aa_seq(dna2, 0));
                                        let d = edit_distance(strme(&aa1), strme(&aa2));
                                        let r1 = if d <= aa1.len() { aa1.len() - d } else { 0 };
                                        let r1 = r1 as f64 / aa1.len() as f64;
                                        let r2 = if d <= aa2.len() { aa2.len() - d } else { 0 };
                                        let r2 = r2 as f64 / aa2.len() as f64;
                                        if r1 >= min_r || r2 >= min_r {
                                            ee.join(i1 as i32, i2 as i32);
                                            continue 'next_light;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let mut reps = Vec::<i32>::new();
                ee.orbit_reps(&mut reps);
                for i in 0..reps.len() {
                    let mut o = Vec::<i32>::new();
                    ee.orbit(i as i32, &mut o);
                    let mut p = Vec::<usize>::new();
                    for j in 0..o.len() {
                        p.push(g[o[j] as usize]);
                    }
                    groups2.push(p);
                }
            }
            groups = groups2;
        }

        // Group by cdr3_aa_heavy_pc.

        if ctl.clono_group_opt.cdr3_aa_heavy_pc.is_some() {
            let min_r = ctl.clono_group_opt.cdr3_aa_heavy_pc.unwrap() / 100.0;
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut ee: EquivRel = EquivRel::new(g.len() as i32);
                for i1 in 0..g.len() {
                    'next_heavy_cdr3: for i2 in i1 + 1..g.len() {
                        if ee.class_id(i1 as i32) == ee.class_id(i2 as i32) {
                            continue;
                        }
                        let (g1, g2) = (g[i1], g[i2]);
                        for u1 in exacts[g1].iter() {
                            for u2 in exacts[g2].iter() {
                                let (ex1, ex2) = (&exact_clonotypes[*u1], &exact_clonotypes[*u2]);
                                for p1 in 0..ex1.share.len() {
                                    if !ex1.share[p1].left {
                                        continue;
                                    }
                                    let aa1 = &ex1.share[p1].cdr3_aa;
                                    for p2 in 0..ex2.share.len() {
                                        if !ex2.share[p2].left {
                                            continue;
                                        }
                                        let aa2 = &ex2.share[p2].cdr3_aa;
                                        let d = edit_distance(aa1, aa2);
                                        let r1 = if d <= aa1.len() { aa1.len() - d } else { 0 };
                                        let r1 = r1 as f64 / aa1.len() as f64;
                                        let r2 = if d <= aa2.len() { aa2.len() - d } else { 0 };
                                        let r2 = r2 as f64 / aa2.len() as f64;
                                        if r1 >= min_r || r2 >= min_r {
                                            ee.join(i1 as i32, i2 as i32);
                                            continue 'next_heavy_cdr3;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let mut reps = Vec::<i32>::new();
                ee.orbit_reps(&mut reps);
                for i in 0..reps.len() {
                    let mut o = Vec::<i32>::new();
                    ee.orbit(i as i32, &mut o);
                    let mut p = Vec::<usize>::new();
                    for j in 0..o.len() {
                        p.push(g[o[j] as usize]);
                    }
                    groups2.push(p);
                }
            }
            groups = groups2;
        }

        // Group by cdr3_aa_light_pc.

        if ctl.clono_group_opt.cdr3_aa_light_pc.is_some() {
            let min_r = ctl.clono_group_opt.cdr3_aa_light_pc.unwrap() / 100.0;
            let mut groups2 = Vec::<Vec<usize>>::new();
            for g in groups.iter() {
                let mut ee: EquivRel = EquivRel::new(g.len() as i32);
                for i1 in 0..g.len() {
                    'next_light_cdr3: for i2 in i1 + 1..g.len() {
                        if ee.class_id(i1 as i32) == ee.class_id(i2 as i32) {
                            continue;
                        }
                        let (g1, g2) = (g[i1], g[i2]);
                        for u1 in exacts[g1].iter() {
                            for u2 in exacts[g2].iter() {
                                let (ex1, ex2) = (&exact_clonotypes[*u1], &exact_clonotypes[*u2]);
                                for p1 in 0..ex1.share.len() {
                                    if ex1.share[p1].left {
                                        continue;
                                    }
                                    let aa1 = &ex1.share[p1].cdr3_aa;
                                    for p2 in 0..ex2.share.len() {
                                        if ex2.share[p2].left {
                                            continue;
                                        }
                                        let aa2 = &ex2.share[p2].cdr3_aa;
                                        let d = edit_distance(aa1, aa2);
                                        let r1 = if d <= aa1.len() { aa1.len() - d } else { 0 };
                                        let r1 = r1 as f64 / aa1.len() as f64;
                                        let r2 = if d <= aa2.len() { aa2.len() - d } else { 0 };
                                        let r2 = r2 as f64 / aa2.len() as f64;
                                        if r1 >= min_r || r2 >= min_r {
                                            ee.join(i1 as i32, i2 as i32);
                                            continue 'next_light_cdr3;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let mut reps = Vec::<i32>::new();
                ee.orbit_reps(&mut reps);
                for i in 0..reps.len() {
                    let mut o = Vec::<i32>::new();
                    ee.orbit(i as i32, &mut o);
                    let mut p = Vec::<usize>::new();
                    for j in 0..o.len() {
                        p.push(g[o[j] as usize]);
                    }
                    groups2.push(p);
                }
            }
            groups = groups2;
        }

        // Join based on grouping.  Stupid, see next step.

        for g in groups.iter() {
            for i in 0..g.len() - 1 {
                e.join(g[i] as i32, g[i + 1] as i32);
            }
        }

        // Get orbit reps.

        let mut greps = Vec::<i32>::new();
        e.orbit_reps(&mut greps);

        // Gather groups and sort so that larger groups (as measured by cells) come first.

        let mut groups = Vec::<Vec<(i32, String)>>::new();
        let mut grepsn = Vec::<usize>::new();
        for i in 0..greps.len() {
            let mut o = Vec::<i32>::new();
            e.orbit(greps[i], &mut o);
            if o.len() < ctl.clono_group_opt.min_group {
                continue;
            }
            let mut z = Vec::<(i32, String)>::new();
            for j in 0..o.len() {
                z.push((o[j], String::new()));
            }
            groups.push(z);
            let mut n = 0;
            for j in 0..o.len() {
                let x = o[j] as usize;
                let s = &exacts[x];
                for k in 0..s.len() {
                    n += exact_clonotypes[s[k]].clones.len();
                }
            }
            grepsn.push(n);
        }
        sort_sync2(&mut grepsn, &mut groups);
        groups.reverse();
        groups

    // ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

    // Case 2: asymmetric grouping.
    } else {
        // Define the center.

        let mut center = Vec::<usize>::new();
        for i in 0..exacts.len() {
            if in_center[i] {
                /*
                let mut d1 = false;
                let s = &exacts[i];
                for k in 0..s.len() {
                    let ex = &exact_clonotypes[s[k]];
                    for u in 0..ex.clones.len() {
                        if ctl.origin_info.donor_id[ex.clones[u][0].dataset_index] == "d1" {
                            d1 = true;
                        }
                    }
                }
                if d1 {
                    center.push(i);
                }
                */
                center.push(i);
            }
        }

        // Define the distance from center clonotypes to other clonotypes.

        let infinity = 1_000_000_000.0_f64;
        let mut dist = Vec::<(usize, Vec<f64>)>::new();
        for i in 0..center.len() {
            dist.push((i, vec![infinity; exacts.len()]));
        }
        dist.par_iter_mut().for_each(|res| {
            let i = res.0;
            for j in 0..exacts.len() {
                let (c1, c2) = (&exacts[center[i]], &exacts[j]);
                for k1 in 0..c1.len() {
                    for k2 in 0..c2.len() {
                        let (ex1, ex2) = (&exact_clonotypes[c1[k1]], &exact_clonotypes[c2[k2]]);
                        let (mut heavy, mut light) = (infinity, infinity);
                        for m1 in 0..ex1.share.len() {
                            let cdr3_aa1 = &ex1.share[m1].cdr3_aa;
                            for m2 in 0..ex2.share.len() {
                                let cdr3_aa2 = &ex2.share[m2].cdr3_aa;
                                if ex1.share[m1].left && ex2.share[m2].left {
                                    let x = edit_distance(cdr3_aa1, cdr3_aa2) as f64;
                                    heavy = heavy.min(x);
                                }
                                if !ex1.share[m1].left && !ex2.share[m2].left {
                                    let x = edit_distance(cdr3_aa1, cdr3_aa2) as f64;
                                    light = light.min(x);
                                }
                            }
                        }
                        res.1[j] = res.1[j].min(heavy + light);
                    }
                }
            }
        });

        // Compute the groups.

        let mut groups = Vec::<Vec<(i32, String)>>::new();
        let bound = &ctl.clono_group_opt.asymmetric_dist_bound;
        let mut top_dist = None;
        if bound.starts_with("top=") {
            top_dist = Some(bound.after("top=").force_usize());
        }
        let mut max_dist = None;
        if bound.starts_with("max=") {
            max_dist = Some(bound.after("max=").force_f64());
        }
        for i in 0..center.len() {
            let mut g = Vec::<(i32, String)>::new();
            g.push((center[i] as i32, "distance = 0".to_string()));
            let mut id = Vec::<(f64, usize)>::new();
            for j in 0..exacts.len() {
                id.push((dist[i].1[j], j));
            }
            id.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for j in 0..id.len() {
                if top_dist.is_some() && j > top_dist.unwrap() {
                    break;
                }
                let d = id[j].0;
                if max_dist.is_some() && d > max_dist.unwrap() {
                    break;
                }
                if id[j].1 != center[i] {
                    g.push((id[j].1 as i32, format!("distance = {}", d)));
                }
            }
            if g.len() >= ctl.clono_group_opt.min_group {
                groups.push(g);
            }
        }
        groups
    }
}
