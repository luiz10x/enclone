// Copyright (c) 2021 10X Genomics, Inc. All rights reserved.

use enclone_core::defs::*;
use enclone_core::join_one::*;
use equiv::EquivRel;
use std::collections::HashMap;

pub fn join_core(
    is_bcr: bool,
    i: usize,
    j: usize,
    ctl: &EncloneControl,
    exact_clonotypes: &Vec<ExactClonotype>,
    info: &Vec<CloneInfo>,
    to_bc: &HashMap<(usize, usize), Vec<String>>,
    sr: &Vec<Vec<f64>>,
    pot: &mut Vec<PotentialJoin>,
) {
    let mut eq: EquivRel = EquivRel::new((j - i) as i32);
    for k1 in i..j {
        for k2 in k1 + 1..j {
            // Do nothing if join could have no effect on equivalence relation.
            // For certain samples, this hugely reduces run time.  That is the purpose of
            // having the equivalence relation.  Observed on MALT samples including 83808.
            // MALT is a B cell cancer in which j-i is very large and in fact the number of
            // exact subclonotypes in one clonotype is very large.

            if !ctl.force && (eq.class_id((k1 - i) as i32) == eq.class_id((k2 - i) as i32)) {
                continue;
            }
            if join_one(is_bcr, k1, k2, ctl, exact_clonotypes, info, to_bc, sr, pot) {
                eq.join((k1 - i) as i32, (k2 - i) as i32);
            }
        }
    }
}
