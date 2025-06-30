//! This module the implementation of VisualGraph, which is the data-structure
//! that we use for assigning (x,y) locations to all of the shapes and edges.
//! The VisualGraph uses a DAG to represent the relationships between the nodes
//! and the Ranks data-structure to represent rows of shapes that have the same
//! x coordinate.

#[cfg(feature = "log")]
extern crate log;

use crate::adt::dag::*;
use crate::core::base::Orientation;
use crate::core::color::Color;
use crate::core::format::RenderBackend;
use crate::core::format::Renderable;
use crate::core::format::Visible;
use crate::core::geometry::Point;
use crate::core::geometry::Position;
use crate::core::style::StyleAttr;
use crate::std_shapes::render::*;
use crate::std_shapes::shapes::*;
use crate::topo::optimizer::EdgeCrossOptimizer;
use crate::topo::optimizer::RankOptimizer;
use std::collections::HashMap;
use std::mem::swap;
use std::vec;

use super::placer::Placer;

#[derive(Debug)]
pub struct VisualGraph {
    // Holds all of the elements in the graph.
    nodes: Vec<Element>,
    pub(crate) subgraphs: Vec<Element>,
    // The arrows and the list of elements that they visits.
    edges: Vec<(Arrow, Vec<NodeHandle>, Vec<UnitHandle>)>,
    // Contains a list of self-edges. We use this as a temporary storage during
    // lowering. This list should be removes by the time we start the layout
    // process.
    self_edges: Vec<(Arrow, NodeHandle, UnitHandle)>,
    // Representing the connections between the nodes. Used to keep the graph
    // a dag by detecting reverse edges. Used to create 'levels', and decide
    // which node moves/controls which node. After lowering, the graph should
    // only contain edges that skip zero or one levels.
    pub dag: DAG,
    // Sets the graph orientation (L-to-R, or T-to-B).
    orientation: Orientation,
}

impl VisualGraph {
    pub fn new(orientation: Orientation) -> Self {
        VisualGraph {
            nodes: Vec::new(),
            subgraphs: Vec::new(),
            edges: Vec::new(),
            self_edges: Vec::new(),
            dag: DAG::new(),
            orientation,
        }
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    pub fn num_nodes(&self) -> usize {
        self.dag.len()
    }

    pub fn iter_nodes(&self) -> NodeIterator {
        self.dag.iter()
    }

    pub fn succ(&self, node: NodeHandle) -> &Vec<NodeHandle> {
        self.dag.successors(node)
    }

    pub fn preds(&self, node: NodeHandle) -> &Vec<NodeHandle> {
        self.dag.predecessors(node)
    }

    pub fn pos(&self, n: NodeHandle) -> Position {
        self.element(n).position()
    }

    pub fn pos_mut(&mut self, n: NodeHandle) -> &mut Position {
        self.element_mut(n).position_mut()
    }

    pub fn is_connector(&self, n: NodeHandle) -> bool {
        return self.element(n).is_connector();
    }

    pub fn transpose(&mut self) {
        for node in self.dag.iter() {
            self.element_mut(node).transpose();
        }
    }

    pub fn element(&self, node: NodeHandle) -> &Element {
        &self.nodes[node.get_index()]
    }

    pub fn element_mut(&mut self, node: NodeHandle) -> &mut Element {
        &mut self.nodes[node.get_index()]
    }
    /// Add a node to the graph.
    /// \returns a handle to the node.
    pub fn add_node(
        &mut self,
        elem: Element,
        idx: SubgraphHandle,
    ) -> NodeHandle {
        let res = self.dag.new_node(idx);
        assert!(res.get_index() == self.nodes.len());
        self.nodes.push(elem);
        res
    }

    /// Add a node to the graph.
    /// \returns a handle to the node.
    // pub fn add_node_old(&mut self, elem: Element) -> NodeHandle {
    //     let res = self.dag.new_node(SubgraphHandle{idx:0});
    //     assert!(res.get_index() == self.nodes.len());
    //     self.nodes.push(elem);
    //     res
    // }

    /// Add an edge to the graph.
    pub fn add_edge(
        &mut self,
        arrow: Arrow,
        from: NodeHandle,
        from_u: UnitHandle,
        to: NodeHandle,
        to_u: UnitHandle,
    ) {
        assert!(from.get_index() < self.nodes.len(), "Invalid handle");
        assert!(to.get_index() < self.nodes.len(), "Invalid handle");
        let lst = vec![from, to];
        let lst_u = vec![from_u, to_u];
        self.edges.push((arrow, lst, lst_u));
    }

    pub fn add_subgraph(
        &mut self,
        orientation: Orientation,
        name: String,
        idx: SubgraphHandle,
    ) -> SubgraphHandle {
        let elem = Element::create_subgraph(
            orientation,
            name,
            &StyleAttr::new(Color::fast("black"), 1, None, 0, 0),
        );
        let res = self.dag.new_subgraph(idx);
        assert!(res.get_index() == self.subgraphs.len());
        self.subgraphs.push(elem);
        res
    }
}

// Render.
impl VisualGraph {
    fn render(&self, debug: bool, rb: &mut dyn RenderBackend) {
        // Draw the nodes.
        for node in &self.nodes {
            node.render(debug, rb);
        }

        // Draw the arrows:
        for arrow in &self.edges {
            let mut elements = Vec::new();
            for h in &arrow.1 {
                elements.push(self.nodes[h.get_index()].clone());
            }
            render_arrow(rb, debug, &elements[..], &arrow.0);
        }

        // Draw the subgraphs.
        for subgraph in &self.subgraphs {
            subgraph.render(debug, rb);
            println!("rendering subgraph: {:?}", subgraph.position());
        }
    }
}

impl VisualGraph {
    pub fn do_it(
        &mut self,
        debug_mode: bool,
        disable_opt: bool,
        disable_layout: bool,
        rb: &mut dyn RenderBackend,
    ) {
        self.lower(disable_opt);
        self.dag.last_rank_expansion();
        println!("Last rank expansion done.");
        Placer::new(self).layout(disable_layout);
        self.render(debug_mode, rb);
    }

    fn lower(&mut self, disable_optimizations: bool) {
        #[cfg(feature = "log")]
        log::info!("Lowering a graph with {} nodes.", self.num_nodes());
        self.to_valid_dag();
        self.split_text_edges();
        self.expand(SubgraphHandle::new(0), disable_optimizations);
        println!("Expanding subgraph 0 done.");

        for elem in self.dag.iter() {
            self.element_mut(elem).resize();
        }
    }

    fn expand(
        &mut self,
        subgraph_idx: SubgraphHandle,
        disable_optimizations: bool,
    ) {
        // expand subgraph into main rank
        let child_subgraphs =
            self.dag.subgraphs[subgraph_idx.idx].subgraphs.clone();
        for i in child_subgraphs.iter() {
            self.expand(*i, disable_optimizations);
        }

        self.split_long_edges(subgraph_idx, disable_optimizations);

        let order = self.dag.topological_sort_from_rank(subgraph_idx);

        self.dag.subgraphs[subgraph_idx.idx].ranks.clear();

        let mut placed = HashMap::new();
        for l in order.iter() {
            match l {
                UnitHandle::Node(n) => {
                    // If the level is a node then we add it to the new rank.
                    placed.insert(*n, None);
                }
                UnitHandle::Subgraph(s) => {
                    // go thorugh the rank and include only if it is nodehandle

                    let subgraph = &self.dag.subgraphs[s.idx];
                    for elem in subgraph.ranks.iter() {
                        for r in elem.iter() {
                            if let UnitHandle::Node(n) = r {
                                placed.insert(*n, None);
                            } else {
                                panic!("Expected a node in the rank, found a subgraph.");
                            }
                        }
                    }
                }
            }
        }
        let mut x_min_idx = 0;
        let mut x_min = usize::MAX;
        let mut y_min_idx = 0;
        let mut y_min = usize::MAX;
        let mut x_max_idx = 0;
        let mut x_max = 0;
        let mut y_max_idx = 0;
        let mut y_max = 0;

        for level in order {
            let (x_offset, y_offset) =
                self.dag
                    .determine_loc_nodehandle(subgraph_idx, level, &placed);
            println!(
                "Placing {:?} at ({}, {})",
                level,
                x_offset,
                y_offset
            );
            match level {
                UnitHandle::Node(n) => {
                    self.dag.add_element_to_rank_subgraph(
                        UnitHandle::new_node(n),
                        y_offset,
                        false,
                        subgraph_idx,
                    );
                    // Mark the node as placed.
                    placed.insert(n, Some((x_offset, y_offset)));
                }
                UnitHandle::Subgraph(s) => {
                    // If the level is a subgraph then we add all of the nodes
                    self.dag.move_subgraph(
                        subgraph_idx,
                        s,
                        x_offset,
                        y_offset,
                        &mut placed,
                    );
                }
            }

            if x_offset < x_min {
                x_min = x_offset;
                x_min_idx = level.get_x_min_idx(&self.dag);
            }
            if y_offset < y_min {
                y_min = y_offset;
                y_min_idx = level.get_y_min_idx(&self.dag);
            }

            if x_offset > x_max {
                x_max = x_offset;
                x_max_idx = level.get_x_max_idx(&self.dag);
            }

            if y_offset > y_max {
                y_max = y_offset;
                y_max_idx = level.get_y_max_idx(&self.dag);
            }
        }

        // self.dag.subgraphs[subgraph_idx.idx].x_min_idx = x_min_idx;
        // self.dag.subgraphs[subgraph_idx.idx].y_min_idx = y_min_idx;
        // self.dag.subgraphs[subgraph_idx.idx].x_max_idx = x_max_idx;
        // self.dag.subgraphs[subgraph_idx.idx].y_max_idx = y_max_idx;
        self.dag.subgraphs[subgraph_idx.idx].x_0 = x_min;
        self.dag.subgraphs[subgraph_idx.idx].y_0 = y_min;
        self.dag.subgraphs[subgraph_idx.idx].width = x_max - x_min + 30;
        self.dag.subgraphs[subgraph_idx.idx].height = y_max - y_min + 1;
        // println!("Subgraph {}: x_min_idx = {}, y_min_idx = {}, x_max_idx = {}, y_max_idx = {}",
        //     subgraph_idx.idx, x_min_idx, y_min_idx, x_max_idx, y_max_idx
        // );
        // println!("width = {}, height = {}",
        //     self.dag.subgraphs[subgraph_idx.idx].width,
        //     self.dag.subgraphs[subgraph_idx.idx].height
        // );

        // now resize subgraph 
        self.subgraphs[subgraph_idx.idx].position_mut().set_size(
            Point::new(
                self.dag.subgraphs[subgraph_idx.idx].width as f64,
                self.dag.subgraphs[subgraph_idx.idx].height as f64,
            ),
        );
        self.subgraphs[subgraph_idx.idx].resize();
        // self.subgraph_into_rectangular(subgraph_idx);
    }

    /// Flip the edges in the graph to create a valid dag.
    /// This is the first step of graph canonicalization.
    pub fn to_valid_dag(&mut self) {
        let edges = self.edges.clone();
        self.edges.clear();

        // At this point the DAG should have all of the nodes, but none of the
        // edges. In here we construct the edges.
        assert_eq!(self.nodes.len(), self.dag.len(), "bad number of nodes");

        // For each edge.
        for edge in edges {
            let mut arrow = edge.0;
            let lst = edge.1;
            let lst_u = edge.2;
            assert_eq!(lst.len(), 2);
            let mut from = lst[0];
            let mut to = lst[1];
            let mut from_u = lst_u[0];
            let mut to_u = lst_u[1];

            if from == to {
                self.self_edges.push((arrow, from, from_u));
                continue;
            }

            // Reverse back edges.
            if self.dag.is_reachable(to, from) {
                swap(&mut from, &mut to);
                arrow = arrow.reverse();
            }
            if self.dag.is_reachable_u(to_u, from_u) {
                swap(&mut from_u, &mut to_u);
            }

            self.dag.add_edge(from, to, from_u, to_u);
            self.add_edge(arrow, from, from_u, to, to_u);

            self.dag.verify();
        }
    }

    /// Convert all of the edges that contain text labels to edges that go
    /// through connectors.
    /// This is the second step of graph canonicalization.
    pub fn split_text_edges(&mut self) {
        let mut edges = self.edges.clone();
        //self.edge_list.clear();

        for edge in edges.iter_mut() {
            let lst = &edge.1;
            let lst_u = &edge.2;
            assert_eq!(lst.len(), 2);
            let arrow = &edge.0;
            let from = lst[0];
            let to = lst[1];
            let from_u = lst_u[0];
            let to_u = lst_u[1];

            // If the edge is empty then there is nothing to do.
            if edge.0.text.is_empty() {
                continue;
            }

            let text = arrow.text.clone();

            let subgraph_idx = self.dag.subgraph_from_unit_handle(from_u);
            // let subgraph_idx = UnitHandle::subgraph_from_uraverse_handle(from_u);

            // Create a new connection block.
            let dir = self.element(from).orientation;
            let conn = Element::create_connector(&text, &arrow.look, dir);
            let conn = self.add_node(conn, subgraph_idx);
            let conn_u = UnitHandle::new_node(conn);

            // Update the edge node list, and remove the text.
            edge.1 = vec![from, conn, to];
            edge.0.text = String::new();

            // Add the edge to dag.
            let res = self.dag.remove_edge(from, to);
            assert!(res, "Expected the edge to be in the graph!");
            self.dag.add_edge(from, conn, from_u, to_u);
            self.dag.add_edge(conn, to, conn_u, to_u);
        }

        self.edges = edges;
    }

    pub fn split_long_edges(
        &mut self,
        subgraph_idx: SubgraphHandle,
        disable_optimizations: bool,
    ) {
        // Assign optimal rank to nodes in the graph.
        self.dag.recompute_node_ranks(subgraph_idx);
        self.dag.verify();
        if !disable_optimizations {
            RankOptimizer::new(&mut self.dag).optimize(subgraph_idx);
        }

        let mut edges = self.edges.clone();
        self.edges.clear();

        for edge in edges.iter_mut() {
            let mut to_skip = false;
            for i in edge.2.iter() {
                match i {
                    UnitHandle::Subgraph(_) => {
                        to_skip = true;
                        break;
                    }
                    UnitHandle::Node(_) => {}
                }
            }
            if to_skip {
                // If the edge contains a subgraph then we skip it.
                continue;
            }

            let mut lst = edge.1.clone();
            let mut lst_u = edge.2.clone();
            let subgraph_idx_edge =
                self.dag.subgraph_from_unit_handle(lst_u[0]);
            if subgraph_idx_edge != subgraph_idx {
                // If the edge is not in the current subgraph then we skip it.
                continue;
            }
            // Points the 'to' edge in each pair in the graph. We start with
            // node '1', and compare to the previous node.
            let mut i = 1;
            while i < lst.len() {
                let prev = lst[i - 1];
                let curr = lst[i];
                let prev_u = lst_u[i - 1];
                let curr_u = lst_u[i];

                let prev_level = self.dag.level_subgraph(prev_u);
                let curr_level = self.dag.level_subgraph(curr_u);

                // If the edges point to a lower rank then move on.
                assert!(prev_level < curr_level, "Invalid edge");
                if prev_level + 1 == curr_level {
                    i += 1;
                    continue;
                }

                // We need to add a new connector node.
                let dir = self.element(prev).orientation;
                let conn = Element::empty_connector(dir);
                let conn = self.add_node(conn, subgraph_idx);
                let conn_u = UnitHandle::new_node(conn);
                lst.insert(i, conn);
                lst_u.insert(i, conn_u);

                // Update the dag connections.
                self.dag.remove_edge(prev, curr);
                self.dag.add_edge(prev, conn, prev_u, conn_u);
                self.dag.add_edge(conn, curr, conn_u, curr_u);

                // Place the new connection node at the right level.
                self.dag.update_node_rank_level(
                    conn_u,
                    prev_level + 1,
                    None,
                    subgraph_idx,
                );
            }

            edge.1 = lst;
            edge.2 = lst_u;
        }
        self.edges = edges;

        if !disable_optimizations {
            EdgeCrossOptimizer::new(&mut self.dag).optimize(subgraph_idx);
        }
        self.expand_self_edges(subgraph_idx);
    }

    pub fn subgraph_into_rectangular(&mut self, subgraph_idx: SubgraphHandle) {
        // Make the subgraph rectangular.
        let width = self.dag.subgraphs[subgraph_idx.idx].ranks.width();
        let height = self.dag.subgraphs[subgraph_idx.idx].ranks.height();

        // add empty nodes to the each level until it reaches width
        for i in 0..height {
            let rank_width =
                self.dag.subgraphs[subgraph_idx.idx].ranks.row(i).len();
            let num_nodes = width - rank_width;
            for _ in 0..num_nodes {
                // Create an empty node.
                let elem = Element::placeholder(self.orientation);
                let empty_node = self.add_node(elem, subgraph_idx);
                let empty_u = UnitHandle::new_node(empty_node);
                self.dag
                    .update_node_rank_level(empty_u, i, None, subgraph_idx);
            }
        }
    }

    /// Convert all of the saved self edges into proper edges in the graph.
    pub fn expand_self_edges(&mut self, subgraph_idx: SubgraphHandle) {
        for se in self.self_edges.clone().iter() {
            let mut arrow = se.0.clone();
            let node = se.1;
            let node_u = se.2;
            let level = self.dag.level(node);
            let text = arrow.text.to_string();
            arrow.text = String::new();
            let dir = self.element(node).orientation;
            let conn = Element::create_connector(&text, &arrow.look, dir);
            let conn = self.add_node(conn, subgraph_idx);
            let conn_u = UnitHandle::new_node(conn);
            self.dag.update_node_rank_level(
                conn_u,
                level,
                Some(node_u),
                subgraph_idx,
            );
            self.edges.push((
                arrow,
                vec![node, conn, node],
                vec![node_u, conn_u, node_u],
            ));
        }

        // Wipe out the self edges.
        self.self_edges.clear();
    }
}
