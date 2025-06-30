//! This module contains the implementation of the placer, which assigns the
//! final (x,y) coordinates to all of the elements in the graph.

#[cfg(feature = "log")]
extern crate log;

use crate::adt::dag::SubgraphHandle;
use crate::core::geometry::Point;
use crate::topo::layout::VisualGraph;
use crate::topo::placer::bk::BK;
use crate::topo::placer::edge_fixer;
use crate::topo::placer::move_between_rows;
use crate::topo::placer::simple;
use crate::topo::placer::verifier;

use crate::core::format::Visible;


#[derive(Debug)]
pub struct Placer<'a> {
    vg: &'a mut VisualGraph,
}

impl<'a> Placer<'a> {
    pub fn new(vg: &'a mut VisualGraph) -> Self {
        Self { vg }
    }

    pub fn layout(&mut self, no_layout: bool) {
        #[cfg(feature = "log")]
        log::info!("Starting layout of {} nodes. ", self.vg.num_nodes());

        // We implement left-to-right layout by transposing the graph.
        let need_transpose = !self.vg.orientation().is_top_to_bottom();
        if need_transpose {
            #[cfg(feature = "log")]
            log::info!("Placing nodes in Left-to-right mode.");
            self.vg.transpose();
        } else {
            #[cfg(feature = "log")]
            log::info!("Placing nodes in Top-to-Bottom mode.");
        }

        move_between_rows::do_it(self.vg);

        // Adjust the boxes within the line (along y) and assign consecutive X
        // coordinates.
        simple::do_it(self.vg);
        println!("After simple placer");

        // Check that the spacial order of the blocks matches the order in the
        // rank.
        verifier::do_it(self.vg);
        println!("After verifier");

        if no_layout {
            #[cfg(feature = "log")]
            log::info!("Skipping the layout phase.");
            // Finalize left-to-right graphs.
            if need_transpose {
                self.vg.transpose();
            }
            return;
        }

        BK::new(self.vg).do_it();
        println!("After BK");

        verifier::do_it(self.vg);
        println!("After verifier");

        edge_fixer::do_it(self.vg);
        println!("After edge fixer");

        assign_x_coordinates_to_subgraphs(self.vg, SubgraphHandle::new(0));

        // Finalize left-to-right graphs.
        if need_transpose {
            self.vg.transpose();
        }
    }
}


pub fn assign_x_coordinates_to_subgraphs(
    vg: &mut VisualGraph,
    subgraph_idx: SubgraphHandle
) {

    let child_subgraphs = vg.dag.subgraphs[subgraph_idx.get_index()].subgraphs.clone();
    for s in child_subgraphs.iter() {
        assign_x_coordinates_to_subgraphs(vg, *s);
    }
    let subgraph = &mut vg.dag.subgraphs[subgraph_idx.get_index()];
    let mut x_min = f64::MAX;
    let mut x_max = f64::MIN;
    // go between y_min and y_max and extract x_min and x_max
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    let child_nodes = subgraph.nodes.clone();
    for n in child_nodes.iter() {
        x_min = x_min.min(vg.pos(*n).bbox_with_half_halo().0.x);
        x_max = x_max.max(vg.pos(*n).bbox_with_half_halo().1.x);

        y_min = y_min.min(vg.pos(*n).bbox_with_half_halo().0.y);
        y_max = y_max.max(vg.pos(*n).bbox_with_half_halo().1.y);
    }

    for s in child_subgraphs.iter() {
        x_min = x_min.min(vg.subgraphs[s.get_index()].position().bbox_with_half_halo().0.x);
        x_max = x_max.max(vg.subgraphs[s.get_index()].position().bbox_with_half_halo().1.x);
        
        y_min = y_min.min(vg.subgraphs[s.get_index()].position().bbox_with_half_halo().0.y);
        y_max = y_max.max(vg.subgraphs[s.get_index()].position().bbox_with_half_halo().1.y);
    }

    let x_center = (x_min + x_max) / 2.0;
    let y_center = (y_min + y_max) / 2.0;
    let width = x_max - x_min;
    let height = y_max - y_min;


    let subgraph_elem = &mut vg.subgraphs[subgraph_idx.get_index()];
    subgraph_elem.position_mut()
        .set_new_center_point(Point::new(x_center, y_center));
    subgraph_elem.position_mut().align_to_center();

    subgraph_elem.position_mut().set_size(
        Point::new(width, height),
    );
    subgraph_elem.resize();

}
