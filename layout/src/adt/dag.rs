//! This module implements the Ranked-DAG data structure. I's a data structure
//! that represents the edges between nodes in the dag as well as the leveling
//! of the nodes. A rank is the ordering of some nodes along the x-axis. Users
//! of this data structure may change the leveling of nodes, and the only
//! guarantee is that the nodes are assigned to some level.

pub(crate) fn find_common_subgraph(
    from: &Vec<SubgraphHandle>,
    from_node: NodeHandle,
    to: &Vec<SubgraphHandle>,
    to_node: NodeHandle,
) -> (UnitHandle, UnitHandle) {
    // Find the first common subgraph.
    let mut subgraph_idx = 0;
    for i in 0..from.len().min(to.len()) {
        if from[i] == to[i] {
            subgraph_idx = i;
        } else {
            break;
        }
    }
    let from_output = if subgraph_idx == from.len() - 1 {
        UnitHandle::new_node(from_node)
    } else {
        UnitHandle::new_subgraph(from[subgraph_idx + 1].clone())
    };
    let to_output = if subgraph_idx == to.len() - 1 {
        UnitHandle::new_node(to_node)
    } else {
        UnitHandle::new_subgraph(to[subgraph_idx + 1].clone())
    };
    (from_output, to_output)
}

use std::{cmp, collections::HashMap};
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Debug)]
pub enum UnitHandle {
    /// A handle to a node.
    Node(NodeHandle),
    /// A handle to a subgraph.
    Subgraph(SubgraphHandle),
}

impl UnitHandle {
    pub(crate) fn new_node(node: NodeHandle) -> Self {
        UnitHandle::Node(node)
    }
    pub(crate) fn new_subgraph(subgraph: SubgraphHandle) -> Self {
        UnitHandle::Subgraph(subgraph)
    }
    pub(crate) fn successors(&self, dag: &DAG) -> Vec<UnitHandle> {
        match self {
            UnitHandle::Node(node) => dag.nodes[node.idx].successors_u.clone(),
            UnitHandle::Subgraph(subgraph) => {
                dag.subgraphs[subgraph.idx].successors_u.clone()
            }
        }
    }
    pub(crate) fn predecessors(&self, dag: &DAG) -> Vec<UnitHandle> {
        match self {
            UnitHandle::Node(node) => {
                dag.nodes[node.idx].predecesssors_u.clone()
            }
            UnitHandle::Subgraph(subgraph) => {
                dag.subgraphs[subgraph.idx].predecesssors_u.clone()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraverseRank {
    _ranks: Vec<Vec<UnitHandle>>,
    _levels: HashMap<UnitHandle, usize>,
}

impl TraverseRank {
    pub fn height(&self) -> usize {
        self._ranks.len()
    }

    pub fn width(&self) -> usize {
        if self._ranks.is_empty() {
            return 0;
        }
        self._ranks.iter().map(|row| row.len()).max().unwrap_or(0)
    }

    pub fn push_row(&mut self, row: Vec<UnitHandle>) {
        self._ranks.push(row);
    }

    pub fn new() -> Self {
        TraverseRank {
            _ranks: Vec::new(),
            _levels: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self._ranks.clear();
        self._levels.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<UnitHandle>> {
        self._ranks.iter()
    }

    pub fn row(&self, idx: usize) -> &Vec<UnitHandle> {
        assert!(idx < self._ranks.len(), "Invalid rank index");
        &self._ranks[idx]
    }

    pub fn row_mut(&mut self, idx: usize) -> &mut Vec<UnitHandle> {
        assert!(idx < self._ranks.len(), "Invalid rank index");
        &mut self._ranks[idx]
    }

    pub fn level(&self, handle: UnitHandle) -> usize {
        *self._levels.get(&handle).unwrap()
    }

    pub fn push(&mut self, level: usize, handle: UnitHandle, prepend: bool) {
        if prepend {
            self._ranks[level].insert(0, handle);
        } else {
            self._ranks[level].push(handle);
        }
        self._levels.insert(handle, level);
    }

    pub fn insert_before(
        &mut self,
        level: usize,
        handle: UnitHandle,
        marker: UnitHandle,
        prepend: bool,
    ) {
        assert!(level < self._ranks.len(), "Invalid rank index");
        let row = &mut self._ranks[level];
        for i in 0..row.len() {
            if row[i] == marker {
                if prepend {
                    row.insert(i, handle);
                } else {
                    row.insert(i + 1, handle);
                }
                self._levels.insert(handle, level);
                return;
            }
        }
        panic!("Can't find the marker node in the array");
    }
}

#[derive(Debug, Clone)]
pub struct Subgraph {
    nodes: Vec<NodeHandle>,
    pub(crate) subgraphs: Vec<SubgraphHandle>,
    parent_subgraph_idx: SubgraphHandle,
    successors_u: Vec<UnitHandle>,
    predecesssors_u: Vec<UnitHandle>,
    pub(crate) ranks: TraverseRank,
}

#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Debug)]
pub struct SubgraphHandle {
    pub(crate) idx: usize,
}

impl SubgraphHandle {
    pub fn new(x: usize) -> Self {
        SubgraphHandle { idx: x }
    }
    pub fn get_index(&self) -> usize {
        self.idx
    }
}

/// The Ranked-DAG data structure.
#[derive(Debug)]
pub struct DAG {
    /// A list of nodes in the dag.
    nodes: Vec<Node>,

    /// Places nodes in levels.
    ranks: RankType,

    /// levels info
    levels: Vec<usize>,

    /// Perform validation checks.
    validate: bool,
    /// A list of subgraphs in the dag.
    pub(crate) subgraphs: Vec<Subgraph>,
}

/// Used by users to keep track of nodes that are saved in the DAG.
#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Debug)]
pub struct NodeHandle {
    idx: usize,
}

impl NodeHandle {
    pub fn new(x: usize) -> Self {
        NodeHandle { idx: x }
    }
    pub fn get_index(&self) -> usize {
        self.idx
    }
}

impl From<usize> for NodeHandle {
    fn from(idx: usize) -> Self {
        NodeHandle { idx }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Node {
    // Points to other edges.
    pub(crate) parent_subgraph_idx: SubgraphHandle,
    pub(crate) successors: Vec<NodeHandle>,
    pub(crate) predecessors: Vec<NodeHandle>,
    pub(crate) successors_u: Vec<UnitHandle>,
    pub(crate) predecesssors_u: Vec<UnitHandle>,
}

pub type RankType = Vec<Vec<NodeHandle>>;

impl Node {
    pub fn new(subgraph_idx: SubgraphHandle) -> Self {
        Node {
            successors: Vec::new(),
            predecessors: Vec::new(),
            successors_u: Vec::new(),
            predecesssors_u: Vec::new(),
            parent_subgraph_idx: subgraph_idx,
        }
    }
}

/// Node iterator for iterating over nodes in the graph.
#[derive(Debug)]
pub struct NodeIterator {
    curr: usize,
    last: usize,
}

impl Iterator for NodeIterator {
    type Item = NodeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr == self.last {
            return None;
        }

        let item = Some(NodeHandle::from(self.curr));
        self.curr += 1;
        item
    }
}

impl DAG {
    pub fn new() -> Self {
        DAG {
            nodes: Vec::new(),
            ranks: Vec::new(),
            levels: Vec::new(),
            subgraphs: Vec::new(),
            validate: true,
        }
    }

    pub fn set_validate(&mut self, validate: bool) {
        self.validate = validate;
    }

    pub(crate) fn last_rank_expansion(&mut self) {
        self.ranks.clear();
        for (i, elem) in self.subgraphs[0].ranks.iter().enumerate() {
            let mut new_rank = Vec::new();
            for h in elem.iter() {
                if let UnitHandle::Node(n) = h {
                    new_rank.push(*n);
                    self.levels[n.get_index()] = i as usize;
                } else {
                    panic!("Expected a node in the rank, found a subgraph.");
                }
            }
            self.ranks.push(new_rank);
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.ranks.clear();
        self.levels.clear();
    }

    pub fn iter(&self) -> NodeIterator {
        NodeIterator {
            curr: 0,
            last: self.nodes.len(),
        }
    }

    pub fn determine_loc_nodehandle(
        &mut self,
        subgraph_idx: SubgraphHandle,
        unit_handle: UnitHandle,
        placed: &HashMap<NodeHandle, Option<(usize, usize)>>,
    ) -> (usize, usize) {
        match unit_handle {
            UnitHandle::Node(node) => {
                let preds = self.predecessors(node);
                for p in preds {
                    if let Some(placed_node) = placed.get(&p) {
                        if let Some((_x, mut y)) = placed_node {
                            // We found a placed predecessor.
                            // Now we can calculate the offset.
                            y += 1;
                            // push ranks until y is avaiblale
                            let ranks =
                                &mut self.subgraphs[subgraph_idx.idx].ranks;
                            while ranks.height() <= y {
                                ranks.push_row(Vec::new());
                            }
                            // now increase x until we find an empty spot in the rank
                            let x = ranks.row(y).clone().len();
                            return (x, y);
                        }
                    }
                }
            }
            UnitHandle::Subgraph(subgraph) => {
                let subgraph = &self.subgraphs[subgraph.idx];
                let child_rank = subgraph.ranks.clone();
                let parent_rank = &mut self.subgraphs[subgraph_idx.idx].ranks;
                for row in child_rank.iter() {
                    for elem in row.iter() {
                        if let UnitHandle::Node(node) = elem {
                            // go thourhg predecessors and find the first placed node
                            let node = self.nodes[node.idx].clone();
                            for pred in node.predecessors.iter() {
                                if let Some(placed_node) = placed.get(pred) {
                                    if let Some((_x, mut y)) = placed_node {
                                        // We found a placed predecessor.
                                        // Now we can calculate the offset.
                                        y += 1;
                                        // let rank = &mut self.subgraphs[subgraph_idx.idx].ranks;
                                        // push ranks until y is avaiblale
                                        while parent_rank.height() <= y {
                                            parent_rank.push_row(Vec::new());
                                        }
                                        // now increase x until we find an empty spot in the rank
                                        let x =
                                            parent_rank.row(y).clone().len();
                                        return (x, y);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // If we reach here, it means that no predecessors were placed.
        // use the next available position.
        let x = self.subgraphs[subgraph_idx.idx].ranks.width();
        (x, 0)
    }

    pub fn parent_subgraph_idx(&self, node: NodeHandle) -> SubgraphHandle {
        self.nodes[node.idx].parent_subgraph_idx
    }

    pub fn subgraph_from_unit_handle(
        &self,
        handle: UnitHandle,
    ) -> SubgraphHandle {
        match handle {
            UnitHandle::Node(node) => self.nodes[node.idx].parent_subgraph_idx,
            UnitHandle::Subgraph(subgraph) => {
                self.subgraphs[subgraph.idx].parent_subgraph_idx
            }
        }
    }

    pub fn add_edge(
        &mut self,
        from: NodeHandle,
        to: NodeHandle,
        from_t: UnitHandle,
        to_t: UnitHandle,
    ) {
        self.nodes[from.idx].successors.push(to);
        self.nodes[to.idx].predecessors.push(from);

        match from_t {
            UnitHandle::Node(node) => {
                self.nodes[node.idx].successors_u.push(to_t);
            }
            UnitHandle::Subgraph(subgraph) => {
                self.subgraphs[subgraph.idx].successors_u.push(to_t);
            }
        }

        match to_t {
            UnitHandle::Node(node) => {
                self.nodes[node.idx].predecesssors_u.push(from_t);
            }
            UnitHandle::Subgraph(subgraph) => {
                self.subgraphs[subgraph.idx].predecesssors_u.push(from_t);
            }
        }
    }
    /// Remove an edge from \p from to \p to.
    /// \returns True if an edge was removed.
    pub fn remove_edge(&mut self, from: NodeHandle, to: NodeHandle) -> bool {
        let succ = &mut self.nodes[from.idx].successors;
        let mut removed_succ = false;

        if let Some(pos) = succ.iter().position(|x| *x == to) {
            succ.remove(pos);
            removed_succ = true;
        }

        let pred = &mut self.nodes[to.idx].predecessors;
        let mut removed_pred = false;
        if let Some(pos) = pred.iter().position(|x| *x == from) {
            pred.remove(pos);
            removed_pred = true;
        }

        // We must preserve the invariant that the pred-succ list must always
        // be up to date.
        assert_eq!(removed_pred, removed_succ);
        removed_pred
    }

    /// Create a new node.
    pub fn new_node(&mut self, subgraph_idx: SubgraphHandle) -> NodeHandle {
        self.nodes.push(Node::new(subgraph_idx));
        self.levels.push(0);

        let node = NodeHandle::new(self.nodes.len() - 1);
        self.subgraphs[subgraph_idx.idx].nodes.push(node);
        self.add_element_to_rank_subgraph(
            UnitHandle::new_node(node),
            0,
            false,
            subgraph_idx,
        );
        node
    }

    pub fn new_subgraph(
        &mut self,
        subgraph_idx: SubgraphHandle,
    ) -> SubgraphHandle {
        let subgraph = Subgraph {
            nodes: Vec::new(),
            subgraphs: Vec::new(),
            successors_u: Vec::new(),
            predecesssors_u: Vec::new(),
            parent_subgraph_idx: subgraph_idx,
            ranks: TraverseRank::new(),
        };
        self.subgraphs.push(subgraph);
        let subgraph_handle = SubgraphHandle::new(self.subgraphs.len() - 1);
        if self.subgraphs.len() > 1 {
            self.subgraphs[subgraph_idx.idx]
                .subgraphs
                .push(subgraph_handle);
            self.add_element_to_rank_subgraph(
                UnitHandle::new_subgraph(subgraph_handle),
                0,
                false,
                subgraph_idx,
            );
        }
        SubgraphHandle::new(self.subgraphs.len() - 1)
    }

    /// Create \p n new nodes.
    pub fn new_nodes(&mut self, n: usize) {
        for _ in 0..n {
            self.nodes.push(Node::new(SubgraphHandle::new(0)));
            self.levels.push(0);
            let node = NodeHandle::new(self.nodes.len() - 1);
            self.add_element_to_rank_subgraph(
                UnitHandle::new_node(node),
                0,
                false,
                SubgraphHandle::new(0),
            );
        }
        self.verify();
    }

    pub fn successors(&self, from: NodeHandle) -> &Vec<NodeHandle> {
        &self.nodes[from.idx].successors
    }

    pub fn predecessors(&self, from: NodeHandle) -> &Vec<NodeHandle> {
        &self.nodes[from.idx].predecessors
    }

    pub fn successors_u(&self, from: UnitHandle) -> Vec<UnitHandle> {
        from.successors(self)
    }

    pub fn predecessors_u(&self, from: UnitHandle) -> Vec<UnitHandle> {
        from.predecessors(self)
    }

    pub fn predecessors_mut(
        &mut self,
        from: NodeHandle,
    ) -> &mut Vec<NodeHandle> {
        &mut self.nodes[from.idx].predecessors
    }

    pub fn successors_mut(&mut self, from: NodeHandle) -> &mut Vec<NodeHandle> {
        &mut self.nodes[from.idx].successors
    }

    pub fn single_pred(&self, from: NodeHandle) -> Option<NodeHandle> {
        if self.nodes[from.idx].predecessors.len() == 1 {
            return Some(self.nodes[from.idx].predecessors[0]);
        }
        None
    }

    pub fn single_succ(&self, from: NodeHandle) -> Option<NodeHandle> {
        if self.nodes[from.idx].successors.len() == 1 {
            return Some(self.nodes[from.idx].successors[0]);
        }
        None
    }

    pub fn verify(&self) {
        if self.validate {
            // Check that the node indices are valid.
            for node in &self.nodes {
                for edge in &node.successors {
                    assert!(edge.idx < self.nodes.len());
                }
            }

            // Check that the graph is a DAG.
            for (i, node) in self.nodes.iter().enumerate() {
                let from = NodeHandle::from(i);
                for dest in node.successors.iter() {
                    let reachable =
                        self.is_reachable(*dest, from) && from != *dest;
                    assert!(!reachable, "We found a cycle!");
                }
            }

            // Make sure that all of the nodes are in ranks.
            assert!(self.count_nodes_in_ranks() <= self.len());
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// \returns True if the node \to is reachable from the node \p from.
    /// This internal method is used for the verification of the graph.
    fn is_reachable_inner(
        &self,
        from: NodeHandle,
        to: NodeHandle,
        visited: &mut Vec<bool>,
    ) -> bool {
        if from == to {
            return true;
        }

        // Don't step into a cycle.
        if visited[from.idx] {
            return false;
        }

        // Push to the dfs stack.
        visited[from.idx] = true;

        let from_node = &self.nodes[from.idx];
        for edge in &from_node.successors {
            if self.is_reachable_inner(*edge, to, visited) {
                return true;
            }
        }

        // Pop from the dfs stack.
        visited[from.idx] = false;
        false
    }

    fn is_reachable_inner_u(
        &self,
        from: UnitHandle,
        to: UnitHandle,
        visited: &mut HashMap<UnitHandle, bool>,
    ) -> bool {
        if from == to {
            return true;
        }

        // Don't step into a cycle.
        if let Some(&visited) = visited.get(&from) {
            if visited {
                return false;
            }
        }

        // Push to the dfs stack.
        visited.insert(from, true);

        let successors = from.successors(self);
        for edge in successors.iter() {
            if self.is_reachable_inner_u(*edge, to, visited) {
                return true;
            }
        }

        // Pop from the dfs stack.
        visited.insert(from, false);
        false
    }

    /// \returns True if there is a path from \p 'from' to \p 'to'.
    pub fn is_reachable(&self, from: NodeHandle, to: NodeHandle) -> bool {
        if from == to {
            return true;
        }

        let mut visited = Vec::new();
        visited.resize(self.nodes.len(), false);
        self.is_reachable_inner(from, to, &mut visited)
    }

    pub fn is_reachable_u(&self, from: UnitHandle, to: UnitHandle) -> bool {
        if from == to {
            return true;
        }

        let mut visited = HashMap::new();
        visited.insert(from, false);
        self.is_reachable_inner_u(from, to, &mut visited)
    }

    /// Return the topological sort order of the nodes in the dag.
    /// This is implemented as the reverse post order scan.
    pub(crate) fn topological_sort_from_rank(
        &self,
        subgraph_idx: SubgraphHandle,
    ) -> Vec<UnitHandle> {
        // A list of vectors in post-order.
        let mut order = Vec::new();

        // // Marks that a node is in the worklist.
        // let mut visited = Vec::new();
        // visited.resize(self.nodes.len(), false);
        // create HASHMAP for visited nodes.
        let mut visited = HashMap::new();

        // A tuple of handle, and command:
        // true- force push.
        // false- this is a child to visit.
        let mut worklist: Vec<(UnitHandle, bool)> = Vec::new();

        // Add all of the values that we want to compute into the worklist.
        // for n in self.iter() {
        //     worklist.push((n, false));
        // }
        let subgraph = &self.subgraphs[subgraph_idx.idx];

        for n in subgraph.nodes.iter() {
            let node_unit_handle = UnitHandle::new_node(n.clone());
            if node_unit_handle.predecessors(self).is_empty() {
                worklist.push((node_unit_handle, false));
            }

            visited.insert(node_unit_handle, false);
        }
        for subgraph in subgraph.subgraphs.iter() {
            let subgraph_unit_handle =
                UnitHandle::new_subgraph(subgraph.clone());
            if subgraph_unit_handle.predecessors(self).is_empty() {
                worklist.push((subgraph_unit_handle, false));
            }
            visited.insert(subgraph_unit_handle, false);
        }

        let rank = &subgraph.ranks;

        // turn rank into vector
        let mut rank_vec = Vec::new();
        for row in rank.iter() {
            for elem in row.iter() {
                rank_vec.push(elem.clone());
            }
        }

        // order worklist by the order in which they appear in rank_vec
        // worklist.sort_by(|a, b| {
        //     let a_idx = rank_vec.iter().position(|x| *x == a.0).unwrap();
        //     let b_idx = rank_vec.iter().position(|x| *x == b.0).unwrap();
        //     a_idx.cmp(&b_idx)
        // });
        // worklist = rank_vec.iter().map(|x| (*x, false)).collect::<Vec<_>>();
        // worklist.reverse();

        while let Some((current, cmd)) = worklist.pop() {
            // Handle 'push' commands.
            if cmd {
                order.push(current);
                continue;
            }

            // Don't visit visited nodes.
            if *visited.get(&current).unwrap() {
                continue;
            }

            visited.insert(current, true);

            // Save this node after all of the children are handles.
            worklist.push((current, true));

            // order successors wrt order in which they appear in rank_vec
            let mut successors = current.successors(self);

            successors.sort_by(|a, b| {
                let a_idx = rank_vec.iter().position(|x| *x == *a).unwrap();
                let b_idx = rank_vec.iter().position(|x| *x == *b).unwrap();
                a_idx.cmp(&b_idx)
            });
            // successors.reverse();

            for edge in successors.iter() {
                // worklist.push((*edge, false));
                // if it Subgraph, check if it is already in the list
                if let UnitHandle::Subgraph(_subgraph) = edge {
                    // Check if the subgraph is already in the worklist.
                    if !worklist.iter().any(|(h, _)| h == edge) {
                        worklist.push((*edge, false));
                    }
                } else {
                    worklist.push((*edge, false));
                }
            }
        }

        // Turn the post-order to a reverse post order.
        order.reverse();
        order
    }
    pub(crate) fn topological_sort_subgraph(
        &self,
        subgraph_idx: SubgraphHandle,
    ) -> Vec<UnitHandle> {
        // A list of vectors in post-order.
        let mut order = Vec::new();

        // // Marks that a node is in the worklist.
        // let mut visited = Vec::new();
        // visited.resize(self.nodes.len(), false);
        // create HASHMAP for visited nodes.
        let mut visited = HashMap::new();

        // A tuple of handle, and command:
        // true- force push.
        // false- this is a child to visit.
        let mut worklist: Vec<(UnitHandle, bool)> = Vec::new();

        // Add all of the values that we want to compute into the worklist.
        // for n in self.iter() {
        //     worklist.push((n, false));
        // }
        let subgraph = &self.subgraphs[subgraph_idx.idx];

        for n in subgraph.nodes.iter() {
            let node_unit_handle = UnitHandle::new_node(n.clone());
            // worklist.push((node_unit_handle, false));
            // add workliks only if it has no predecessors
            if node_unit_handle.predecessors(self).is_empty() {
                worklist.push((node_unit_handle, false));
            }
            visited.insert(node_unit_handle, false);
        }
        for subgraph in subgraph.subgraphs.iter() {
            let subgraph_unit_handle =
                UnitHandle::new_subgraph(subgraph.clone());
            // worklist.push((subgraph_unit_handle, false));
            // add workliks only if it has no predecessors
            if subgraph_unit_handle.predecessors(self).is_empty() {
                worklist.push((subgraph_unit_handle, false));
            }
            visited.insert(subgraph_unit_handle, false);
        }

        while let Some((current, cmd)) = worklist.pop() {
            // Handle 'push' commands.
            if cmd {
                order.push(current);
                continue;
            }

            // Don't visit visited nodes.
            if *visited.get(&current).unwrap() {
                continue;
            }

            visited.insert(current, true);

            // Save this node after all of the children are handles.
            worklist.push((current, true));

            let successors = current.successors(self);
            for edge in successors.iter() {
                // worklist.push((*edge, false));
                // if it Subgraph, check if it is already in the list
                if let UnitHandle::Subgraph(_subgraph) = edge {
                    // Check if the subgraph is already in the worklist.
                    if !worklist.iter().any(|(h, _)| h == edge) {
                        worklist.push((*edge, false));
                    }
                } else {
                    worklist.push((*edge, false));
                }
            }
        }

        // Turn the post-order to a reverse post order.
        order.reverse();
        order
    }

    pub(crate) fn move_subgraph(
        &mut self,
        parent_subgraph_idx: SubgraphHandle,
        subgraph_idx: SubgraphHandle,
        _x_offset: usize,
        y_offset: usize,
        placed: &mut HashMap<NodeHandle, Option<(usize, usize)>>,
    ) {
        // Move the subgraph by the given offset.
        let subgraph = self.subgraphs[subgraph_idx.idx].clone();
        for row in subgraph.ranks.iter() {
            for elem in row.clone() {
                if let UnitHandle::Node(node) = elem {
                    let level = subgraph.ranks.level(elem);
                    self.add_element_to_rank_subgraph(
                        UnitHandle::new_node(node),
                        level + y_offset,
                        false,
                        parent_subgraph_idx,
                    );
                    let x = self.subgraphs[parent_subgraph_idx.idx]
                        .ranks
                        .row(level + y_offset)
                        .len();
                    placed.insert(node, Some((x, level + y_offset)));
                }
            }
        }
    }

    // The methods below are related to the rank (placing nodes in levels). //

    /// \returns the number of ranks in the dag.
    // fn num_levels(&self) -> usize {
    //     self.ranks.len()
    // }
    pub fn num_levels(&self) -> usize {
        self.ranks.len()
    }
    pub(crate) fn num_levels_subgraph(
        &self,
        subgraph_idx: SubgraphHandle,
    ) -> usize {
        self.subgraphs[subgraph_idx.idx].ranks.height()
    }

    /// \return a mutable reference to a row at level \p level.
    pub fn row_mut(&mut self, level: usize) -> &mut Vec<NodeHandle> {
        assert!(level < self.ranks.len(), "Invalid rank");
        &mut self.ranks[level]
    }

    /// \return a reference to a row at level \p level.
    pub fn row(&self, level: usize) -> &Vec<NodeHandle> {
        assert!(level < self.ranks.len(), "Invalid rank");
        &self.ranks[level]
    }

    pub(crate) fn row_subgraph(
        &self,
        subgraph_idx: SubgraphHandle,
        level: usize,
    ) -> &Vec<UnitHandle> {
        assert!(
            level < self.subgraphs[subgraph_idx.idx].ranks.height(),
            "Invalid rank"
        );
        &self.subgraphs[subgraph_idx.idx].ranks.row(level)
    }

    pub(crate) fn row_subgraph_mut(
        &mut self,
        subgraph_idx: SubgraphHandle,
        level: usize,
    ) -> &mut Vec<UnitHandle> {
        assert!(
            level < self.subgraphs[subgraph_idx.idx].ranks.height(),
            "Invalid rank"
        );
        self.subgraphs[subgraph_idx.idx].ranks.row_mut(level)
    }

    /// \return a reference to the whole rank data structure.
    pub fn ranks(&self) -> &RankType {
        &self.ranks
    }

    /// \return a mutable reference to the whole rank data structure.
    pub fn ranks_mut(&mut self) -> &mut RankType {
        &mut self.ranks
    }

    /// \returns True if \p elem is the first node in the row \p level.
    pub fn is_first_in_row(&self, elem: NodeHandle, level: usize) -> bool {
        if level >= self.ranks.len() || self.ranks[level].is_empty() {
            return false;
        }
        self.ranks[level][0] == elem
    }

    /// \returns True if \p elem is the last node in the row \p level.
    pub fn is_last_in_row(&self, elem: NodeHandle, level: usize) -> bool {
        if level >= self.ranks.len() || self.ranks[level].is_empty() {
            return false;
        }
        let last_idx = self.ranks[level].len() - 1;
        self.ranks[level][last_idx] == elem
    }

    /// Place the element \p elem at the nth level \p level. If the level does
    /// not exist then create it. If \p prepend is set then the node is inserted
    /// at the beginning of the rank. The node must not be in the rank when this
    /// method is called.
    pub(crate) fn add_element_to_rank_subgraph(
        &mut self,
        elem: UnitHandle,
        level: usize,
        prepend: bool,
        subgraph_idx: SubgraphHandle,
    ) {
        while self.subgraphs[subgraph_idx.idx].ranks.height() < level + 1 {
            self.subgraphs[subgraph_idx.idx].ranks.push_row(Vec::new());
        }

        self.subgraphs[subgraph_idx.idx]
            .ranks
            .push(level, elem, prepend);
    }

    /// Places all of the nodes in ranks (levels).
    // pub fn recompute_node_ranks(&mut self) {
    //     assert!(!self.is_empty(), "Sorting an empty graph");
    //     let order = self.topological_sort();
    //     let levels = self.compute_levels(&order);
    //     self.ranks.clear();
    //     for (i, level) in levels.iter().enumerate() {
    //         self.add_element_to_rank(NodeHandle::from(i), *level, false);
    //     }
    // }

    pub fn recompute_node_ranks(&mut self, subgraph_idx: SubgraphHandle) {
        assert!(!self.is_empty(), "Sorting an empty graph");
        let order = self.topological_sort_subgraph(subgraph_idx);
        let levels = self.compute_levels(&order);
        self.subgraphs[subgraph_idx.idx].ranks.clear();

        for o in order.iter() {
            let level = *levels.get(o).unwrap();
            self.add_element_to_rank_subgraph(
                o.clone(),
                level,
                false,
                subgraph_idx,
            );
        }
    }

    /// \returns the number of nodes that are in ranks.
    /// This is used for verification of the dag.
    fn count_nodes_in_ranks(&self) -> usize {
        let mut cnt = 0;
        for row in self.ranks.iter() {
            cnt += row.len();
        }
        cnt
    }

    /// Move the node \p node to a new level \p new_level.
    /// Place the node before \p node, or at the end.
    pub fn update_node_rank_level(
        &mut self,
        node: UnitHandle,
        new_level: usize,
        insert_before: Option<UnitHandle>,
        subgraph_idx: SubgraphHandle,
    ) {
        // Make sure that the row exists.
        while self.subgraphs[subgraph_idx.idx].ranks.height() < new_level + 1 {
            self.subgraphs[subgraph_idx.idx].ranks.push_row(Vec::new());
        }

        if let Option::Some(marker) = insert_before {
            self.subgraphs[subgraph_idx.idx]
                .ranks
                .insert_before(new_level, node, marker, false);
        }

        self.subgraphs[subgraph_idx.idx]
            .ranks
            .push(new_level, node, false);
        assert_eq!(self.level_subgraph(node), new_level);
    }

    pub fn level_subgraph(&self, handle: UnitHandle) -> usize {
        let subgraph_idx = match handle {
            UnitHandle::Node(node) => self.nodes[node.idx].parent_subgraph_idx,
            UnitHandle::Subgraph(subgraph) => {
                self.subgraphs[subgraph.idx].parent_subgraph_idx
            }
        };
        self.subgraphs[subgraph_idx.idx].ranks.level(handle)
    }

    /// \returns the level of the node \p node in the rank.
    pub fn level(&self, node: NodeHandle) -> usize {
        assert!(node.get_index() < self.len(), "Node not in the dag");
        self.levels[node.get_index()]
    }

    pub(crate) fn compute_levels(
        &self,
        order: &[UnitHandle],
    ) -> HashMap<UnitHandle, usize> {
        // let mut levels: Vec<usize> = Vec::new();
        let mut levels = HashMap::new();
        // assert_eq!(order.len(), self.nodes.len());
        // Levels has the same layout as the DAG node list.
        for node in order.iter() {
            levels.insert(node.clone(), (0, false));
        }

        // For each node in the order (starting with a node of level zero).
        for src in order {
            // Update the level of all successors.
            let successors = src.successors(self);
            for dest in successors.iter() {
                // Ignore self edges.
                if src == dest {
                    continue;
                }
                match dest {
                    UnitHandle::Node(_n) => {
                        levels.insert(
                            dest.clone(),
                            (
                                cmp::max(
                                    levels.get(src).unwrap().0 + 1,
                                    levels.get(dest).unwrap().0,
                                ),
                                true,
                            ),
                        );
                    }
                    UnitHandle::Subgraph(_) => {
                        // if field 1 true dont change
                        let current_level = levels.get(dest).unwrap();
                        if !current_level.1 {
                            levels.insert(
                                dest.clone(),
                                (
                                    cmp::max(
                                        levels.get(src).unwrap().0 + 1,
                                        current_level.0,
                                    ),
                                    true,
                                ),
                            );
                        }
                    }
                }
            }
        }

        // TODO: this needs modification after subgraph
        // For each node in the order.
        // for src in order {
        //     for dest in self.nodes[src.idx].successors.iter() {
        //         assert!(levels[dest.idx] >= levels[src.idx]);
        //     }
        // }

        let mut level_final = HashMap::new();
        // Convert the levels to a vector of pairs.
        for (handle, level) in levels.iter() {
            level_final.insert(handle.clone(), level.0);
        }
        level_final
    }
}

impl Default for DAG {
    fn default() -> Self {
        Self::new()
    }
}

// #[test]
// fn test_simple_construction() {
//     let mut g = DAG::new();
//     //  TODO: needs modification after subgraph
//     // let h0 = g.new_node();
//     // g.verify();

//     // let h1 = g.new_node();
//     // let h2 = g.new_node();
//     // let h3 = g.new_node();
//     // let h4 = g.new_node();

//     // assert_ne!(h0, h1);
//     // assert_ne!(h1, h2);

//     // g.add_edge(h0, h1);
//     // g.add_edge(h1, h2);
//     // g.add_edge(h0, h2);
//     // g.add_edge(h2, h3);
//     // g.add_edge(h3, h4);

//     // g.verify();

//     // let order = g.topological_sort();
//     // let levels = g.compute_levels(&order);
//     // assert_eq!(order.len(), g.len());
//     // assert_eq!(levels.len(), g.len());

//     // for i in 0..g.len() {
//     //     println!("{}) node {},  level {}", i, order[i].idx, levels[i]);
//     // }
// }

// #[test]
// fn test_rank_api() {
//     let mut g = DAG::new();
//     // let h0 = g.new_node();
//     // let h1 = g.new_node();
//     // let h2 = g.new_node();

//     // g.add_edge(h0, h1);
//     // g.add_edge(h1, h2);

//     // g.recompute_node_ranks();
//     // g.verify();

//     // assert_eq!(g.level(h0), 0);
//     // assert_eq!(g.level(h1), 1);
//     // assert_eq!(g.level(h2), 2);

//     // let r1 = g.remove_edge(h0, h1);
//     // let r2 = g.remove_edge(h0, h1);
//     // // Should be able to remove the edge that we inserted.
//     // assert!(r1);
//     // // The edge should no longer be there!
//     // assert!(!r2);
// }
