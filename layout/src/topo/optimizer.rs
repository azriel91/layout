//! This module contains optimization passes that transform the graphs in different
//! phases of the program. Here you can find things like optimizations for
//! sinking or hoisting nodes to reduce the number of live edges, and
//! optimizations that move nodes within a row to reduce edge crossing.

use crate::adt::dag::SubgraphHandle;
use crate::adt::dag::UnitHandle;
use crate::adt::dag::DAG;
use crate::core::base::Direction;

/// This optimizations changes the order of nodes within a rank (ordering along
/// the x-axis). The transformation tries to reduce the number of edges that
/// cross each other.
#[derive(Debug)]
pub struct EdgeCrossOptimizer<'a> {
    dag: &'a mut DAG,
}
impl<'a> EdgeCrossOptimizer<'a> {
    pub fn new(dag: &'a mut DAG) -> Self {
        Self { dag }
    }

    /// Given two nodes that may have connections in \p row, check how many of
    /// these edges intersect. Check both successors and predecessors.
    ///               A   B
    ///             /   \/ \
    ///            /    /\  \
    ///  Row: [][][][][][][][][][]
    fn num_crossing(
        &self,
        a: UnitHandle,
        b: UnitHandle,
        row: &[UnitHandle],
    ) -> usize {
        let mut sum = 0;
        // Record the number of edges that previously connected with node B.
        let mut num_b = 0;

        let a_edges1 = self.dag.successors_u(a);
        let a_edges2 = self.dag.predecessors_u(a);
        let b_edges1 = self.dag.successors_u(b);
        let b_edges2 = self.dag.predecessors_u(b);

        for node in row {
            let is_a1 = a_edges1.iter().any(|x| x == node);
            let is_a2 = a_edges2.iter().any(|x| x == node);
            let is_b1 = b_edges1.iter().any(|x| x == node);
            let is_b2 = b_edges2.iter().any(|x| x == node);
            if is_a1 || is_a2 {
                sum += num_b;
            }
            if is_b1 || is_b2 {
                num_b += 1;
            }
        }
        sum
    }

    // Shuffle the nodes in all of the ranks.
    pub fn perturb_rank(&mut self, subgraph_idx: SubgraphHandle) {
        for i in 0..self.dag.num_levels_subgraph(subgraph_idx) {
            // let row = self.dag.row_mut(i);
            let row = self.dag.row_subgraph_mut(subgraph_idx, i);
            let len = row.len();
            for j in 0..len {
                row.swap((j * 17) % len, j);
            }
        }
    }

    // Move the elements in the rank to the left, to perturb the graph.
    pub fn rotate_rank(&mut self, subgraph_idx: SubgraphHandle) {
        for i in 0..self.dag.num_levels_subgraph(subgraph_idx) {
            // let row = self.dag.row_mut(i);
            let row = self.dag.row_subgraph_mut(subgraph_idx, i);
            row.rotate_left(1);
        }
    }

    pub fn optimize(&mut self, subgraph_idx: SubgraphHandle) {
        self.dag.verify();
        #[cfg(feature = "log")]
        log::info!("Optimizing edge crossing.");
        // this is only shuffle no need to update levels
        let mut best_rank = self.dag.subgraphs[subgraph_idx.idx].ranks.clone();
        let mut best_cnt = self.count_crossed_edges(subgraph_idx);
        #[cfg(feature = "log")]
        log::info!("Starting with {} crossings.", best_cnt);
        for i in 0..50 {
            let dir = match i % 4 {
                0 => Direction::Both,
                1 => Direction::Up,
                _ => Direction::Down,
            };
            self.swap_crossed_edges(subgraph_idx, dir);
            let new_cnt = self.count_crossed_edges(subgraph_idx);
            if new_cnt < best_cnt {
                #[cfg(feature = "log")]
                log::info!("Found a rank with {} crossings.", new_cnt);
                best_rank = self.dag.subgraphs[subgraph_idx.idx].ranks.clone();
                best_cnt = new_cnt;
            }

            self.rotate_rank(subgraph_idx);
            if i % 10 == 0 {
                self.perturb_rank(subgraph_idx);
            }
        }
        self.dag.subgraphs[subgraph_idx.idx].ranks = best_rank;
        self.swap_crossed_edges_inter(subgraph_idx, Direction::Both);
    }

    fn count_crossed_edges(&self, subgraph_idx: SubgraphHandle) -> usize {
        let mut sum = 0;
        // Compare each row to the row afterwards.
        for row_idx in 0..self.dag.num_levels_subgraph(subgraph_idx) - 1 {
            let first_row = self.dag.row_subgraph(subgraph_idx, row_idx);
            let second_row = self.dag.row_subgraph(subgraph_idx, row_idx + 1);
            sum += self.count_crossing_in_rows(first_row, second_row);
        }
        sum
    }

    fn count_crossing_in_rows(
        &self,
        first: &[UnitHandle],
        second: &[UnitHandle],
    ) -> usize {
        if first.len() < 2 {
            return 0;
        }
        let mut sum = 0;
        // Check for each pair of nodes a,b where b comes after a.
        for i in 0..first.len() {
            for j in i + 1..first.len() {
                let a = first[i];
                let b = first[j];
                sum += self.num_crossing(a, b, second);
            }
        }
        sum
    }

    /// Scan all of the node pairs in the module and count the number of crossed
    /// edges. If \p allow_swap is set then swap the edges if it reduces the
    /// number of crossing.
    fn swap_crossed_edges(
        &mut self,
        subgraph_idx: SubgraphHandle,
        dir: Direction,
    ) {
        let mut changed = true;
        while changed {
            changed = false;
            if dir.is_down() {
                for i in 0..self.dag.num_levels_subgraph(subgraph_idx) {
                    changed |=
                        self.swap_crossed_edges_on_row(subgraph_idx, i, dir);
                }
            }
            if dir.is_up() {
                for i in (0..self.dag.num_levels_subgraph(subgraph_idx)).rev() {
                    changed |=
                        self.swap_crossed_edges_on_row(subgraph_idx, i, dir);
                }
            }
        }
    }

    fn swap_crossed_edges_inter(
        &mut self,
        subgraph_idx: SubgraphHandle,
        dir: Direction,
    ) {
        let mut changed = true;
        while changed {
            changed = false;
            if dir.is_down() {
                for i in 0..self.dag.num_levels_subgraph(subgraph_idx) {
                    changed |=
                        self.swap_crossed_edges_on_row_inter(subgraph_idx, i);
                }
            }
            if dir.is_up() {
                for i in (0..self.dag.num_levels_subgraph(subgraph_idx)).rev() {
                    changed |=
                        self.swap_crossed_edges_on_row_inter(subgraph_idx, i);
                }
            }
        }
    }

    /// See swap_crossed_edges.
    fn swap_crossed_edges_on_row(
        &mut self,
        subgraph_idx: SubgraphHandle,
        row_idx: usize,
        dir: Direction,
    ) -> bool {
        let mut changed = false;

        let num_rows = self.dag.num_levels_subgraph(subgraph_idx);

        let prev_row = if row_idx > 0 && dir.is_up() {
            self.dag.row_subgraph(subgraph_idx, row_idx - 1).clone()
        } else {
            Vec::new()
        };
        let next_row = if row_idx + 1 < num_rows && dir.is_down() {
            self.dag.row_subgraph(subgraph_idx, row_idx + 1).clone()
        } else {
            Vec::new()
        };

        let mut row = self.dag.row_subgraph(subgraph_idx, row_idx).clone();

        if row.len() < 2 {
            return false;
        }

        // For each two consecutive elements in the row:
        for i in 0..row.len() - 1 {
            let a = row[i];
            let b = row[i + 1];

            let mut ab = 0;
            let mut ba = 0;
            // Figure out if A crosses the edges of B, and vice versa, on both
            // the edges pointing up and down.
            ab += self.num_crossing(a, b, &prev_row);
            ba += self.num_crossing(b, a, &prev_row);
            ab += self.num_crossing(a, b, &next_row);
            ba += self.num_crossing(b, a, &next_row);

            // Swap the edges.
            if ab > ba {
                row[i] = b;
                row[i + 1] = a;
                changed = true;
            }
        }

        if changed {
            // *self.dag.row_mut(row_idx) = row;
            *self.dag.row_subgraph_mut(subgraph_idx, row_idx) = row;
        }
        changed
    }

    /// See swap_crossed_edges.
    fn swap_crossed_edges_on_row_inter(
        &mut self,
        subgraph_idx: SubgraphHandle,
        row_idx: usize,
    ) -> bool {
        let mut changed = false;

        let mut row = self.dag.row_subgraph(subgraph_idx, row_idx).clone();

        if row.len() < 2 {
            return false;
        }

        // For each two consecutive elements in the row:
        for i in 0..row.len() - 1 {
            let a = row[i];
            let b = row[i + 1];

            let delta_crossing = self.optimize_intersubgraph(i, i + 1, &row);
            if 0 > delta_crossing {
                row[i] = b;
                row[i + 1] = a;
                changed = true;
            }
        }

        if changed {
            // *self.dag.row_mut(row_idx) = row;
            *self.dag.row_subgraph_mut(subgraph_idx, row_idx) = row;
        }
        changed
    }

    fn optimize_intersubgraph(
        &self,
        a_idx: usize,
        b_idx: usize,
        row: &[UnitHandle],
    ) -> isize {
        let mut node_indices = Vec::new();
        let mut intersubgraph_edges = Vec::new();
        for i in 0..row.len() {
            for j in i + 1..row.len() {
                if let UnitHandle::Node(n) = row[i] {
                    let n_level =
                        self.dag.level_subgraph(UnitHandle::new_node(n));
                    for p in self.dag.predecessors(n) {
                        if let UnitHandle::Node(m) = row[j] {
                            let m_level = self
                                .dag
                                .level_subgraph(UnitHandle::new_node(m));
                            if *p == m {
                                // Collect the edges that cross the edge from A to B.
                                intersubgraph_edges
                                    .push((n, m, i, j, n_level, m_level));
                            }
                        } else if let UnitHandle::Subgraph(s) = row[j] {
                            // go through rank and add node if it is node
                            for node in self.dag.subgraphs[s.idx]
                                .ranks
                                .iter()
                                .flat_map(|r| r.iter())
                            {
                                let m_level = self.dag.level_subgraph(*node);
                                if let UnitHandle::Node(m) = node {
                                    if p == m {
                                        // Collect the edges that cross the edge from A to B.
                                        intersubgraph_edges.push((
                                            n, *m, i, j, n_level, m_level,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    for s in self.dag.successors(n) {
                        if let UnitHandle::Node(m) = row[j] {
                            let m_level = self
                                .dag
                                .level_subgraph(UnitHandle::new_node(m));
                            if *s == m {
                                // Collect the edges that cross the edge from A to B.
                                intersubgraph_edges
                                    .push((n, m, i, j, n_level, m_level));
                            }
                        } else if let UnitHandle::Subgraph(sub) = row[j] {
                            // go through rank and add node if it is node
                            for node in self.dag.subgraphs[sub.idx]
                                .ranks
                                .iter()
                                .flat_map(|r| r.iter())
                            {
                                let m_level = self.dag.level_subgraph(*node);
                                if let UnitHandle::Node(m) = node {
                                    if s == m {
                                        // Collect the edges that cross the edge from A to B.
                                        intersubgraph_edges.push((
                                            n, *m, i, j, n_level, m_level,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else if let UnitHandle::Subgraph(s) = row[i] {
                    // go through rank and add node if it is node
                    for node in self.dag.subgraphs[s.idx]
                        .ranks
                        .iter()
                        .flat_map(|r| r.iter())
                    {
                        let n_level = self.dag.level_subgraph(*node);
                        if let UnitHandle::Node(n) = node {
                            node_indices.push(n);
                            for p in self.dag.predecessors(*n) {
                                if let UnitHandle::Node(m) = row[j] {
                                    let m_level = self.dag.level_subgraph(
                                        UnitHandle::new_node(m),
                                    );
                                    if *p == m {
                                        // Collect the edges that cross the edge from A to B.
                                        intersubgraph_edges.push((
                                            *n, m, i, j, n_level, m_level,
                                        ));
                                    }
                                } else if let UnitHandle::Subgraph(s) = row[j] {
                                    // go through rank and add node if it is node
                                    for node in self.dag.subgraphs[s.idx]
                                        .ranks
                                        .iter()
                                        .flat_map(|r| r.iter())
                                    {
                                        let m_level =
                                            self.dag.level_subgraph(*node);
                                        if let UnitHandle::Node(m) = node {
                                            if p == m {
                                                // Collect the edges that cross the edge from A to B.
                                                intersubgraph_edges.push((
                                                    *n, *m, i, j, n_level,
                                                    m_level,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            for s in self.dag.successors(*n) {
                                if let UnitHandle::Node(m) = row[j] {
                                    let m_level = self.dag.level_subgraph(
                                        UnitHandle::new_node(m),
                                    );
                                    if *s == m {
                                        // Collect the edges that cross the edge from A to B.
                                        intersubgraph_edges.push((
                                            *n, m, i, j, n_level, m_level,
                                        ));
                                    }
                                } else if let UnitHandle::Subgraph(sub) = row[j]
                                {
                                    // go through rank and add node if it is node
                                    for node in self.dag.subgraphs[sub.idx]
                                        .ranks
                                        .iter()
                                        .flat_map(|r| r.iter())
                                    {
                                        if let UnitHandle::Node(m) = node {
                                            let m_level =
                                                self.dag.level_subgraph(*node);
                                            if s == m {
                                                // Collect the edges that cross the edge from A to B.
                                                intersubgraph_edges.push((
                                                    *n, *m, i, j, n_level,
                                                    m_level,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // now record those that cross each other wrt to levels (last 2 indices of edges)
        let mut crossed = Vec::new();
        for i in 0..intersubgraph_edges.len() {
            for j in i + 1..intersubgraph_edges.len() {
                let a = intersubgraph_edges[i];
                let b = intersubgraph_edges[j];
                // Check if the edges cross each other.
                let b_min = b.4.min(b.5);
                let b_max = b.4.max(b.5);
                let a_min = a.4.min(a.5);
                let a_max = a.4.max(a.5);
                let same_pair_subgraph = a.2 == b.2 && a.3 == b.3;
                if same_pair_subgraph {
                    // If they are the same pair, then we don't count them.
                    // since swapping them wont change the crossing.
                    continue;
                }
                if (b_max > a.4 && a.4 > b_min) || (b_max > a.5 && a.5 > b_min)
                {
                    crossed.push((a, b));
                } else if (a_max > b.4 && b.4 > a_min)
                    || (a_max > b.5 && b.5 > a_min)
                {
                    crossed.push((b, a));
                }
            }
        }

        let mut cross_old = 0;

        for c in crossed.iter() {
            // Count the number of crossing based 2 3 idx of edge
            let b_min = c.1 .2.min(c.1 .3);
            let b_max = c.1 .2.max(c.1 .3);
            let a_min = c.0 .2.min(c.0 .3);
            let a_max = c.0 .2.max(c.0 .3);
            if (b_max > c.0 .2 && c.0 .2 > b_min)
                || (b_max > c.0 .3 && c.0 .3 > b_min)
            {
                cross_old += 1;
            } else if (a_max > c.1 .2 && c.1 .2 > a_min)
                || (a_max > c.1 .3 && c.1 .3 > a_min)
            {
                cross_old += 1;
            }
        }

        // apply idx exchange
        let mut crossed_new = crossed.clone();
        for c in crossed_new.iter_mut() {
            if c.0 .2 == a_idx {
                c.0 .2 = b_idx;
            } else if c.0 .2 == b_idx {
                c.0 .2 = a_idx;
            }
            if c.0 .3 == a_idx {
                c.0 .3 = b_idx;
            } else if c.0 .3 == b_idx {
                c.0 .3 = a_idx;
            }
            if c.1 .2 == a_idx {
                c.1 .2 = b_idx;
            } else if c.1 .2 == b_idx {
                c.1 .2 = a_idx;
            }
            if c.1 .3 == a_idx {
                c.1 .3 = b_idx;
            } else if c.1 .3 == b_idx {
                c.1 .3 = a_idx;
            }
        }

        let mut cross_new = 0;
        for c in crossed_new.iter() {
            // Count the number of crossing based 2 3 idx of edge
            let b_min = c.1 .2.min(c.1 .3);
            let b_max = c.1 .2.max(c.1 .3);
            let a_min = c.0 .2.min(c.0 .3);
            let a_max = c.0 .2.max(c.0 .3);
            if (b_max > c.0 .2 && c.0 .2 > b_min)
                || (b_max > c.0 .3 && c.0 .3 > b_min)
            {
                cross_new += 1;
            } else if (a_max > c.1 .2 && c.1 .2 > a_min)
                || (a_max > c.1 .3 && c.1 .3 > a_min)
            {
                cross_new += 1;
            }
        }

        // If the number of crossing is reduced, then apply the change.
        cross_new - cross_old
    }
}

/// This optimization sinks nodes in an attempt to shorten the length of edges
/// that run through the graph.
#[derive(Debug)]
pub struct RankOptimizer<'a> {
    dag: &'a mut DAG,
}

impl<'a> RankOptimizer<'a> {
    pub fn new(dag: &'a mut DAG) -> Self {
        Self { dag }
    }

    pub fn try_to_sink_node(
        &mut self,
        node: UnitHandle,
        subgraph_idx: SubgraphHandle,
    ) -> bool {
        let backs = node.predecessors(self.dag);
        let fwds = node.successors(self.dag);
        // Don't try to sink if we increase the number of live edges,
        // or if there are no forward edges.
        if backs.len() > fwds.len() || backs.len() + fwds.len() == 0 {
            return false;
        }

        let curr_rank = self.dag.level_subgraph(node);
        let mut highest_next = self.dag.len();
        for elem in fwds {
            let next_rank = self.dag.level_subgraph(elem);
            highest_next = highest_next.min(next_rank);
        }

        // We found an opportunity to sink a node.
        if highest_next > curr_rank + 1 {
            self.dag.update_node_rank_level(
                node,
                highest_next - 1,
                None,
                subgraph_idx,
            );
            return true;
        }
        false
    }

    // Try to sink nodes to shorten the length of edges.
    pub fn optimize(&mut self, subgraph_idx: SubgraphHandle) {
        self.dag.verify();

        #[cfg(feature = "log")]
        log::info!("Optimizing the ranks.");
        #[cfg(feature = "log")]
        let mut cnt = 0;
        #[cfg(feature = "log")]
        let mut iter = 0;

        loop {
            let mut c = 0;
            for node in self.dag.topological_sort_subgraph(subgraph_idx) {
                if self.try_to_sink_node(node, subgraph_idx) {
                    c += 1;
                }
            }
            #[cfg(feature = "log")]
            {
                cnt += c;
                iter += 1;
            }
            if c == 0 {
                break;
            }
        }

        #[cfg(feature = "log")]
        log::info!("Sank {} nodes in {} iteration.", cnt, iter);
    }
}
