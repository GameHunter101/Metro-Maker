use std::{cmp::Ordering, ptr::NonNull};

use rand::prelude::*;

use crate::street_graph::Segment;

type Link = NonNull<Node>;

pub struct SkipList {
    nodes: Link,
    end: Link,
    rng: ThreadRng,
    len: usize,
}

impl SkipList {
    pub fn new() -> Self {
        unsafe {
            let nodes = Node::new_empty_chain();
            Self {
                nodes,
                end: nodes.as_ref().next_ptrs[0],
                rng: rand::rng(),
                len: 0,
            }
        }
    }

    pub fn height(&self) -> usize {
        unsafe { (*self.nodes.as_ptr()).next_ptrs.len() }
    }

    pub fn insert(
        &mut self,
        element: usize,
        segments: &[Segment],
        height: f32,
    ) -> (Option<usize>, Option<usize>) {
        unsafe {
            let (traverse_node, traverse_path) = Node::traverse_level(
                self.nodes,
                (*self.nodes.as_ptr()).next_ptrs.len() - 1,
                element.clone(),
                segments,
                height,
            );

            self.len += 1;

            Node::append(
                self.nodes,
                self.end,
                traverse_node,
                element,
                traverse_path,
                &mut self.rng,
            )
        }
    }

    pub fn remove(&mut self, element: usize, segments: &[Segment], height: f32) -> bool {
        unsafe {
            let (traverse_target, _) = Node::traverse_level(
                self.nodes,
                self.height() - 1,
                element.clone(),
                segments,
                height,
            );

            if (*traverse_target.as_ptr()).node_type.cmp(
                &NodeType::Value(element),
                segments,
                height,
            ) == Ordering::Equal
            {
                let boxed_target = Box::from_raw(traverse_target.as_ptr());

                let node_prev_ptrs = boxed_target.prev_ptrs.clone();
                let node_next_ptrs = boxed_target.next_ptrs.clone();

                assert_eq!(node_prev_ptrs.len(), node_next_ptrs.len());

                for i in 0..node_next_ptrs.len() {
                    (&mut (*node_prev_ptrs[i].as_ptr()).next_ptrs)[i] = node_next_ptrs[i];
                    (&mut (*node_next_ptrs[i].as_ptr()).prev_ptrs)[i] = node_prev_ptrs[i];
                }

                self.len -= 1;

                true
            } else {
                false
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self, level: usize) -> Iter {
        unsafe {
            Iter {
                next: Some(self.nodes.as_ref()),
                level,
            }
        }
    }

    pub fn to_vec(&self) -> Vec<usize> {
        self.iter(0)
            .flat_map(|node_type| {
                if let NodeType::Value(curve_index) = node_type {
                    Some(*curve_index)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Drop for SkipList {
    fn drop(&mut self) {
        unsafe {
            while !(*self.nodes.as_ptr()).next_ptrs.is_empty() {
                let boxed_node = Box::from_raw(self.nodes.as_ptr());
                self.nodes = boxed_node.next_ptrs[0];
            }
            let _ = Box::from_raw(self.nodes.as_ptr());
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    node_type: NodeType,
    next_ptrs: Vec<Link>,
    prev_ptrs: Vec<Link>,
}

impl Node {
    fn new_empty_chain() -> NonNull<Node> {
        unsafe {
            let end_node = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                node_type: NodeType::End,
                next_ptrs: Vec::new(),
                prev_ptrs: Vec::new(),
            })));

            let start_node = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                node_type: NodeType::Start,
                next_ptrs: vec![end_node],
                prev_ptrs: Vec::new(),
            })));

            (*end_node.as_ptr()).prev_ptrs = vec![start_node];

            start_node
        }
    }

    fn traverse_level(
        start: Link,
        level: usize,
        element: usize,
        segments: &[Segment],
        height: f32,
    ) -> (Link, Vec<Link>) {
        unsafe {
            if let Some(next) = start.as_ref().next_ptrs.get(level).copied() {
                if (*next.as_ptr())
                    .node_type
                    .cmp(&NodeType::Value(element), segments, height)
                    == Ordering::Greater
                {
                    if level == 0 {
                        (start, Vec::new())
                    } else {
                        let (node, mut path) =
                            Self::traverse_level(start, level - 1, element, segments, height);
                        path.push(start);
                        (node, path)
                    }
                } else {
                    Self::traverse_level(next, level, element, segments, height)
                }
            } else {
                (start, Vec::new())
            }
        }
    }

    fn append(
        origin: Link,
        end: Link,
        node: Link,
        element: usize,
        traversal_path: Vec<Link>,
        rng: &mut ThreadRng,
    ) -> (Option<usize>, Option<usize>) {
        unsafe {
            let max_possible_height = traversal_path.len() + 1;
            let height = rng.random_range(1..=max_possible_height);

            let all_previous_nodes: Vec<Link> = std::iter::once(node)
                .chain(traversal_path.iter().copied())
                .collect();

            let (prev_ptrs, next_ptrs): (Vec<Link>, Vec<Link>) = (0..height)
                .map(|level| {
                    (
                        all_previous_nodes[level],
                        all_previous_nodes[level].as_ref().next_ptrs[level],
                    )
                })
                .unzip();

            let new_node = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                node_type: NodeType::Value(element),
                next_ptrs: next_ptrs.clone(),
                prev_ptrs: prev_ptrs.clone(),
            })));

            for level in 0..prev_ptrs.len() {
                (&mut (*prev_ptrs[level].as_ptr()).next_ptrs)[level] = new_node;
                (&mut (*next_ptrs[level].as_ptr()).prev_ptrs)[level] = new_node;
            }

            if height == max_possible_height && rng.random() {
                (&mut (*origin.as_ptr()).next_ptrs).push(new_node);
                (&mut (*end.as_ptr()).prev_ptrs).push(new_node);

                (&mut (*new_node.as_ptr()).next_ptrs).push(end);
                (&mut (*new_node.as_ptr()).prev_ptrs).push(origin);
            }

            let prev_neighbor_type = &(*prev_ptrs[0].as_ptr()).node_type;
            let next_neighbor_type = &(*next_ptrs[0].as_ptr()).node_type;

            let prev_neighbor = if let NodeType::Value(val) = prev_neighbor_type {
                Some(*val)
            } else {
                None
            };

            let next_neighbor = if let NodeType::Value(val) = next_neighbor_type {
                Some(*val)
            } else {
                None
            };

            (prev_neighbor, next_neighbor)
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Start,
    Value(usize),
    End,
}
fn get_x_val_of_segment_at_height(segment: &Segment, height: f32) -> f32 {
    segment[0].x
        + ((height - segment[0].y) / (segment[1] - segment[0]).y) * (segment[1] - segment[0]).x
}

impl NodeType {
    fn cmp(&self, other: &NodeType, segments: &[Segment], height: f32) -> Ordering {
        match self {
            NodeType::Start => Ordering::Less,
            NodeType::Value(lhs) => match other {
                NodeType::Start => Ordering::Greater,
                NodeType::Value(rhs) => {
                    if rhs == lhs {
                        Ordering::Equal
                    } else {
                        let lhs_x = get_x_val_of_segment_at_height(&segments[*lhs], height);
                        let rhs_x = get_x_val_of_segment_at_height(&segments[*rhs], height);
                        lhs_x.partial_cmp(&rhs_x).unwrap_or(Ordering::Less)
                    }
                }
                NodeType::End => Ordering::Less,
            },
            NodeType::End => Ordering::Greater,
        }
    }
}

pub struct Iter<'a> {
    next: Option<&'a Node>,
    level: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a NodeType;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.next.take().map(|node| {
                self.next = node.next_ptrs.get(self.level).map(|ptr| ptr.as_ref());
                &node.node_type
            })
        }
    }
}
