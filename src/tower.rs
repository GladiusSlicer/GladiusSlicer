use crate::SlicerErrors;
use gladius_shared::types::{IndexedTriangle, Vertex};
use rayon::prelude::*;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/*

    Rough algoritim

    build tower
        For each point store all edges and face connected to but above it

    progress up tower






*/

/// Calculate the vertex the Line from `v_start` to `v_end` where
/// it intersects with the plane z
///
/// <div class="warning">If v_start.z == v_end.z then divide by 0</div>
///
/// # Arguments
/// * `z` - z height of the resulting point
/// * `v_start` - Starting point of the line
/// * `v_end` - Ending point of the line
#[inline]
fn line_z_intersection(z: f64, v_start: Vertex, v_end: Vertex) -> Vertex {
    let z_normal = (z - v_start.z) / (v_end.z - v_start.z);
    let y = lerp(v_start.y, v_end.y, z_normal);
    let x = lerp(v_start.x, v_end.x, z_normal);
    Vertex { x, y, z }
}

#[inline]
fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + f * (b - a)
}

/// A set of triangles and their associated vertices
pub struct TriangleTower {
    vertices: Vec<Vertex>,
    tower_vertices: Vec<TowerVertex>,
}

impl TriangleTower {
    pub fn from_triangles_and_vertices(
        triangles: &[IndexedTriangle],
        vertices: Vec<Vertex>,
    ) -> Result<Self, SlicerErrors> {
        let mut future_tower_vert: Vec<Vec<TriangleEvent>> =
            (0..vertices.len()).map(|_| vec![]).collect();

        // for each triangle add it to the tower

        for (triangle_index, index_tri) in triangles.iter().enumerate() {
            // index 0 is always lowest
            future_tower_vert[index_tri.verts[0]].push(TriangleEvent::MiddleVertex {
                trailing_edge: index_tri.verts[1],
                leading_edge: index_tri.verts[2],
                triangle: triangle_index,
            });

            // depending what is the next vertex is its either leading or trailing
            if vertices[index_tri.verts[1]] < vertices[index_tri.verts[2]] {
                future_tower_vert[index_tri.verts[1]].push(TriangleEvent::TrailingEdge {
                    trailing_edge: index_tri.verts[2],
                    triangle: triangle_index,
                });
            } else {
                future_tower_vert[index_tri.verts[2]].push(TriangleEvent::LeadingEdge {
                    leading_edge: index_tri.verts[1],
                    triangle: triangle_index,
                });
            }
        }

        // for each triangle event, add it to the lowest vertex and
        // create a list of all vertices and there above edges

        let res_tower_vertices: Vec<TowerVertex> = future_tower_vert
            .into_par_iter()
            .enumerate()
            .map(|(index, events)| {
                let fragments = join_triangle_event(&events, index);
                TowerVertex {
                    start_index: index,
                    next_ring_fragments: fragments,
                }
            })
            .collect();

        // propagate errors
        let mut tower_vertices = res_tower_vertices;

        // sort lowest to highest
        tower_vertices.sort_by(|a, b| {
            vertices[a.start_index]
                .partial_cmp(&vertices[b.start_index])
                .expect("STL ERROR: No Points should have NAN values")
        });

        Ok(Self {
            vertices,
            tower_vertices,
        })
    }

    pub fn get_height_of_vertex(&self, index: usize) -> f64 {
        if index >= self.tower_vertices.len() {
            f64::INFINITY
        } else {
            self.vertices[self.tower_vertices[index].start_index].z
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TowerVertex {
    pub next_ring_fragments: Vec<TowerRing>,
    pub start_index: usize,
}
#[derive(Clone, Debug, PartialEq)]
struct TowerRing {
    elements: Vec<TowerRingElement>,
}

impl TowerRing {
    #[inline]
    fn is_complete_ring(&self) -> bool {
        self.elements.first() == self.elements.last() && self.elements.len() > 3
    }

    fn join_rings(mut first: TowerRing, second: TowerRing) -> Self {
        TowerRing::join_rings_in_place(&mut first, second);

        first
    }

    fn join_rings_in_place(first: &mut TowerRing, second: TowerRing) {
        first.elements.extend_from_slice(&second.elements[1..]);
    }

    fn split_on_edge(self, edge: usize) -> Vec<Self> {
        let mut new_ring = vec![];
        let mut frags = vec![];

        for e in self.elements {
            if let TowerRingElement::Edge { end_index, .. } = e {
                if end_index == edge {
                    frags.push(TowerRing { elements: new_ring });
                    new_ring = vec![];
                } else {
                    new_ring.push(e);
                }
            } else {
                new_ring.push(e);
            }
        }

        if frags.is_empty() {
            //add in the fragment
            frags.push(TowerRing { elements: new_ring });
        } else {
            //append to the beginning to prevent ophaned segments
            if frags[0].elements.is_empty() {
                frags[0].elements = new_ring;
            } else {
                new_ring.extend_from_slice(&frags[0].elements[1..]);
                frags[0].elements = new_ring;
            }
        }

        //remove all fragments that are single sized and faces. They ends with that vertex

        frags.retain(|frag| frag.elements.len() > 1);

        frags
    }
}

impl Display for TowerRing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for e in &self.elements {
            write!(f, "{e} ")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq)]
enum TowerRingElement {
    Face {
        triangle_index: usize,
    },
    Edge {
        start_index: usize,
        end_index: usize,
    },
}

impl Display for TowerRingElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            TowerRingElement::Face { triangle_index, .. } => {
                write!(f, "F{triangle_index} ")
            }
            TowerRingElement::Edge { end_index, .. } => {
                write!(f, "E{end_index} ")
            }
        }
    }
}

impl PartialEq for TowerRingElement {
    fn eq(&self, other: &Self) -> bool {
        match self {
            TowerRingElement::Edge {
                end_index,
                start_index,
                ..
            } => match other {
                TowerRingElement::Edge {
                    end_index: oei,
                    start_index: osi,
                    ..
                } => end_index == oei && start_index == osi,
                TowerRingElement::Face { .. } => false,
            },
            TowerRingElement::Face { triangle_index, .. } => match other {
                TowerRingElement::Face {
                    triangle_index: oti,
                    ..
                } => oti == triangle_index,
                TowerRingElement::Edge { .. } => false,
            },
        }
    }
}

impl Hash for TowerRingElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TowerRingElement::Edge {
                end_index,
                start_index,
                ..
            } => {
                end_index.hash(state);
                start_index.hash(state);
            }
            TowerRingElement::Face { triangle_index, .. } => {
                triangle_index.hash(state);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TriangleEvent {
    MiddleVertex {
        leading_edge: usize,
        triangle: usize,
        trailing_edge: usize,
    },
    LeadingEdge {
        leading_edge: usize,
        triangle: usize,
    },
    TrailingEdge {
        triangle: usize,
        trailing_edge: usize,
    },
}

fn join_triangle_event(events: &[TriangleEvent], starting_point: usize) -> Vec<TowerRing> {
    // debug!("Tri events = {:?}",events);
    let mut element_list: Vec<TowerRing> = events
        .iter()
        .map(|event| match event {
            TriangleEvent::LeadingEdge {
                leading_edge,
                triangle,
            } => {
                let triangle_element = TowerRingElement::Face {
                    triangle_index: *triangle,
                };
                let edge_element = TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *leading_edge,
                };

                TowerRing {
                    elements: vec![edge_element, triangle_element],
                }
            }
            TriangleEvent::TrailingEdge {
                triangle,
                trailing_edge,
            } => {
                let edge_element = TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *trailing_edge,
                };

                let triangle_element = TowerRingElement::Face {
                    triangle_index: *triangle,
                };
                TowerRing {
                    elements: vec![triangle_element, edge_element],
                }
            }
            TriangleEvent::MiddleVertex {
                leading_edge,
                triangle,
                trailing_edge,
            } => {
                let trail_edge_element = TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *trailing_edge,
                };

                let triangle_element = TowerRingElement::Face {
                    triangle_index: *triangle,
                };

                let lead_edge_element = TowerRingElement::Edge {
                    start_index: starting_point,
                    end_index: *leading_edge,
                };
                TowerRing {
                    elements: vec![lead_edge_element, triangle_element, trail_edge_element],
                }
            }
        })
        .collect();

    join_fragments(&mut element_list);

    element_list
}

fn join_fragments(fragments: &mut Vec<TowerRing>) {
    /*

        for frag in &*fragments{
            println!("fragment {}",frag);
        }
    */

    for first_pos in 0..fragments.len() {
        let mut second_pos = first_pos + 1;
        while second_pos < fragments.len() {
            let swap;
            if {
                let first = fragments
                    .get(first_pos)
                    .expect("Index is validated by loop");
                let second = fragments
                    .get(second_pos)
                    .expect("Index is validated by loop");

                swap = second.elements.last() == first.elements.first();
                first.elements.last() == second.elements.first() || swap
            } {
                if swap {
                    fragments.swap(second_pos, first_pos);
                }
                let second_r = fragments.swap_remove(second_pos);
                let first_r = fragments
                    .get_mut(first_pos)
                    .expect("Index is validated by loop");
                TowerRing::join_rings_in_place(first_r, second_r);

                //dont progress as the swap makes this position valid again
            } else {
                second_pos += 1;
            }
            if swap {
                second_pos = first_pos + 1;
            }
        }
    }
}

pub struct TriangleTowerIterator<'s> {
    tower: &'s TriangleTower,
    tower_vert_index: usize,
    z_height: f64,
    active_rings: Vec<TowerRing>,
}

impl<'s> TriangleTowerIterator<'s> {
    pub fn new(tower: &'s TriangleTower) -> Self {
        let z_height = tower.get_height_of_vertex(0);
        Self {
            z_height,
            tower,
            tower_vert_index: 0,
            active_rings: vec![],
        }
    }

    pub fn advance_to_height(&mut self, z: f64) -> Result<(), SlicerErrors> {
        //println!("Advance to height {} {} {}", self.tower.get_height_of_vertex(self.tower_vert_index), z, self.tower.tower_vertices[self.tower_vert_index].start_index);

        while self.tower.get_height_of_vertex(self.tower_vert_index) < z
            && self.tower.tower_vertices.len() + 1 != self.tower_vert_index
        {
            let pop_tower_vert = self.tower.tower_vertices[self.tower_vert_index].clone();

            //Create Frags from rings by removing current edges
            self.active_rings = self
                .active_rings
                .drain(..)
                .flat_map(|tower_ring| {
                    tower_ring
                        .split_on_edge(pop_tower_vert.start_index)
                        .into_iter()
                })
                .collect();
            /*
            trace!("split edge: {:?}", pop_tower_vert.start_index );
            trace!("split frags:");
            for f in &frags{
                trace!("\t{}",f);
            }*/

            //Add the new fragments

            self.active_rings.extend(pop_tower_vert.next_ring_fragments);
            // trace!("all frags:");
            // for f in &frags{
            //     trace!("\t{}",f);
            // }

            join_fragments(&mut self.active_rings);
            // trace!("join frags:");
            // for f in &frags{
            //     trace!("\t{}",f);
            // }
            self.tower_vert_index += 1;

            for ring in &self.active_rings {
                if !ring.is_complete_ring() {
                    return Err(SlicerErrors::TowerGeneration);
                }
            }
        }

        self.z_height = z;

        Ok(())
    }

    pub fn get_points(&self) -> Vec<Vec<Vertex>> {
        self.active_rings
            .iter()
            .map(|ring| {
                let mut points: Vec<Vertex> = ring
                    .elements
                    .iter()
                    .filter_map(|e| {
                        if let TowerRingElement::Edge {
                            start_index,
                            end_index,
                            ..
                        } = e
                        {
                            Some(line_z_intersection(
                                self.z_height,
                                self.tower.vertices[*start_index],
                                self.tower.vertices[*end_index],
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();

                //complete loop
                points.push(points[0]);

                points
            })
            .collect()
    }
}

pub fn create_towers(
    models: &[(Vec<Vertex>, Vec<IndexedTriangle>)],
) -> Result<Vec<TriangleTower>, SlicerErrors> {
    models
        .iter()
        .map(|(vertices, triangles)| {
            TriangleTower::from_triangles_and_vertices(triangles, vertices.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_rings_test() {
        let r1 = TowerRing {
            elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
                TowerRingElement::Face { triangle_index: 0 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 2,
                },
            ],
        };

        let r2 = TowerRing {
            elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 2,
                },
                TowerRingElement::Face { triangle_index: 2 },
                TowerRingElement::Edge {
                    start_index: 4,
                    end_index: 6,
                },
            ],
        };

        let r3 = TowerRing {
            elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
                TowerRingElement::Face { triangle_index: 0 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 2,
                },
                TowerRingElement::Face { triangle_index: 2 },
                TowerRingElement::Edge {
                    start_index: 4,
                    end_index: 6,
                },
            ],
        };

        ring_sliding_equality_assert(&TowerRing::join_rings(r1, r2), &r3);
    }

    #[test]
    fn split_on_edge_test() {
        let r1 = TowerRing {
            elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
                TowerRingElement::Face { triangle_index: 0 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 2,
                },
                TowerRingElement::Face { triangle_index: 2 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
            ],
        };

        let frags = r1.split_on_edge(2);

        let expected = vec![TowerRing {
            elements: vec![
                TowerRingElement::Face { triangle_index: 2 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
                TowerRingElement::Face { triangle_index: 0 },
            ],
        }];
        rings_sliding_equality_assert(frags, expected);
    }

    #[test]
    fn assemble_fragment_simple_test() {
        let mut frags = vec![
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 1,
                    },
                    TowerRingElement::Face { triangle_index: 0 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 2,
                    },
                    TowerRingElement::Face { triangle_index: 2 },
                    TowerRingElement::Edge {
                        start_index: 4,
                        end_index: 6,
                    },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 4,
                        end_index: 6,
                    },
                    TowerRingElement::Face { triangle_index: 2 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 1,
                    },
                ],
            },
        ];

        join_fragments(&mut frags);

        let expected = vec![TowerRing {
            elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
                TowerRingElement::Face { triangle_index: 0 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 2,
                },
                TowerRingElement::Face { triangle_index: 2 },
                TowerRingElement::Edge {
                    start_index: 4,
                    end_index: 6,
                },
                TowerRingElement::Face { triangle_index: 2 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 1,
                },
            ],
        }];

        rings_sliding_equality_assert(frags, expected);
    }
    #[test]
    fn assemble_fragment_multiple_test() {
        let mut frags = vec![
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 1,
                    },
                    TowerRingElement::Face { triangle_index: 0 },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Face { triangle_index: 0 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 2,
                    },
                    TowerRingElement::Face { triangle_index: 1 },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Face { triangle_index: 1 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 3,
                    },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 3,
                    },
                    TowerRingElement::Face { triangle_index: 4 },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Face { triangle_index: 4 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 1,
                    },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 11,
                    },
                    TowerRingElement::Face { triangle_index: 10 },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Face { triangle_index: 10 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 12,
                    },
                    TowerRingElement::Face { triangle_index: 11 },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Face { triangle_index: 11 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 11,
                    },
                ],
            },
        ];

        join_fragments(&mut frags);

        let expected = vec![
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 1,
                    },
                    TowerRingElement::Face { triangle_index: 0 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 2,
                    },
                    TowerRingElement::Face { triangle_index: 1 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 3,
                    },
                    TowerRingElement::Face { triangle_index: 4 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 1,
                    },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 11,
                    },
                    TowerRingElement::Face { triangle_index: 10 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 12,
                    },
                    TowerRingElement::Face { triangle_index: 11 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 11,
                    },
                ],
            },
        ];

        rings_sliding_equality_assert(frags, expected);
    }
    #[test]
    fn assemble_fragment_3_fragment_test() {
        let mut frags = vec![
            TowerRing {
                elements: vec![
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 11,
                    },
                    TowerRingElement::Face { triangle_index: 10 },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Face { triangle_index: 10 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 12,
                    },
                    TowerRingElement::Face { triangle_index: 11 },
                ],
            },
            TowerRing {
                elements: vec![
                    TowerRingElement::Face { triangle_index: 11 },
                    TowerRingElement::Edge {
                        start_index: 0,
                        end_index: 11,
                    },
                ],
            },
        ];

        join_fragments(&mut frags);

        let expected = vec![TowerRing {
            elements: vec![
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 11,
                },
                TowerRingElement::Face { triangle_index: 10 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 12,
                },
                TowerRingElement::Face { triangle_index: 11 },
                TowerRingElement::Edge {
                    start_index: 0,
                    end_index: 11,
                },
            ],
        }];

        rings_sliding_equality_assert(frags, expected);
    }

    fn rings_sliding_equality_assert(lhs: Vec<TowerRing>, rhs: Vec<TowerRing>) {
        if lhs == rhs {
            return;
        }
        if lhs.len() != rhs.len() {
            panic!("ASSERT rings count are different lengths");
        }

        for q in 0..lhs.len() {
            ring_sliding_equality_assert(&lhs[q], &rhs[q])
        }
    }

    fn ring_sliding_equality_assert(lhs: &TowerRing, rhs: &TowerRing) {
        if lhs == rhs {
            return;
        }
        if lhs.elements.len() != rhs.elements.len() {
            panic!("ASSERT ring {} and {} are different lengths", lhs, rhs);
        }

        for q in 0..lhs.elements.len() - 1 {
            let mut equal = true;
            for w in 0..lhs.elements.len() - 1 {
                equal = equal && rhs.elements[w] == lhs.elements[(w + q) % (lhs.elements.len() - 1)]
            }

            if equal {
                return;
            }

            if lhs.elements.len() != rhs.elements.len() {
                panic!("ASSERT ring {} and {} are different", lhs, rhs);
            }
        }
    }
}
