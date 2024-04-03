use {common::HashSet, std::collections::VecDeque};

use crate::rudder::Function;

pub struct LoopAnalysis {
    f: Function,
    contains_loop: bool,
}

impl LoopAnalysis {
    pub fn new(f: &Function) -> Self {
        let mut selph = Self {
            f: f.clone(),
            contains_loop: false,
        };

        selph.analyse();

        selph
    }

    fn analyse(&mut self) {
        let mut work_list = VecDeque::new();
        let mut seen_list = HashSet::default();

        work_list.push_back(self.f.entry_block());

        while !work_list.is_empty() {
            let current = work_list.pop_front().unwrap();

            if seen_list.contains(&current) {
                self.contains_loop = true;
                break;
            }

            work_list.extend(current.targets());
            seen_list.insert(current);
        }
    }

    pub fn contains_loop(&self) -> bool {
        self.contains_loop
    }
}
