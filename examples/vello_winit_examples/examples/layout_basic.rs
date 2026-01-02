use hashbrown::HashMap;
use kurbo::{Affine, Circle, Rect, Size, Stroke, Vec2};
use rectree::layout::{Constraint, LayoutCtx, LayoutSolver};
use rectree::node::RectNode;
use rectree::{NodeId, Rectree};
use vello::Scene;
use vello::peniko::Color;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

const AREA: f64 = 2500.0;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut demo = LayoutDemo::new();

    demo.add_vertical(
        None,
        Color::from_rgb8(200, 200, 10),
        |demo, id| VerticalCenteredList {
            padding: 20.0,
            children: (0..5)
                .map(|_i| {
                    let area = FixedArea {
                        use_width: false,
                        target_area: AREA,
                    };

                    demo.add_area(
                        Some(id),
                        Color::from_rgb8(10, 200, 200),
                        |_, _| area,
                    )
                })
                .collect(),
        },
    );

    let mut app = VelloWinitApp::new(demo);

    event_loop.run_app(&mut app).unwrap();
}

#[derive(Default, Debug)]
struct World {
    areas: HashMap<NodeId, FixedArea>,
    verticals: HashMap<NodeId, VerticalCenteredList>,
    node_colors: HashMap<NodeId, Color>,
}

impl LayoutSolver for World {
    fn constraint(
        &self,
        _id: &NodeId,
        _tree: &Rectree,
    ) -> Constraint {
        Constraint::flexible()
    }

    fn build<F>(
        &self,
        id: &NodeId,
        tree: &Rectree,
        mut set_translation: F,
    ) -> Size
    where
        F: FnMut(NodeId, Vec2),
    {
        if let Some(area) = self.areas.get(id)
            && let Some(node) = tree.try_get(id)
        {
            let constraint = node.constraint();
            return match (constraint.width, constraint.height) {
                (None, None) => {
                    // Square
                    Size::splat(area.target_area.sqrt())
                }
                (None, Some(h)) => Size::new(area.target_area / h, h),
                (Some(w), None) => Size::new(w, area.target_area / w),
                (Some(w), Some(h)) => {
                    if area.use_width {
                        Size::new(w, area.target_area / w)
                    } else {
                        Size::new(area.target_area / h, h)
                    }
                }
            };
        } else if let Some(vertical) = self.verticals.get(id)
            && let Some(node) = tree.try_get(id)
        {
            let width =
                node.constraint().width.unwrap_or_else(|| {
                    let mut max_width = 0.0;

                    for id in vertical.children.iter() {
                        if let Some(node) = tree.try_get(id) {
                            max_width =
                                node.size.width.max(max_width);
                        }
                    }

                    max_width
                }) + vertical.padding * 2.0;

            let mut height = vertical.padding;
            for id in vertical.children.iter() {
                if let Some(node) = tree.try_get(id) {
                    let remainder = width - node.size.width;

                    let x = remainder * 0.5;
                    let y = height;
                    set_translation(*id, Vec2::new(x, y));

                    height += node.size.height + vertical.padding;
                }
            }

            return Size::new(width, height);
        }

        unreachable!("{id:?}")
    }
}

#[derive(Debug, Clone)]
struct VerticalCenteredList {
    padding: f64,
    // In real world scenario, this should just store the ids
    // mapping to an arena of widgets.
    children: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy)]
struct FixedArea {
    /// Use width if constrained on both axis.
    /// (Acts like a text widget.)
    pub use_width: bool,
    pub target_area: f64,
}

// impl Widget for FixedArea {
//     fn constraint(&self) -> Constraint {
//         Constraint::flexible()
//     }

//     fn build<F>(&self, node: &RectNode, _: &Rectree, _: F) -> Size
//     where
//         F: FnMut(NodeId, Vec2),
//     {
//         let constraint = node.constraint();
//         match (constraint.width, constraint.height) {
//             (None, None) => {
//                 // Square
//                 Size::splat(self.target_area.sqrt())
//             }
//             (None, Some(h)) => Size::new(self.target_area / h, h),
//             (Some(w), None) => Size::new(w, self.target_area / w),
//             (Some(w), Some(h)) => {
//                 if self.use_width {
//                     Size::new(w, self.target_area / w)
//                 } else {
//                     Size::new(self.target_area / h, h)
//                 }
//             }
//         }
//     }
// }

// pub trait Widget {
//     fn constraint(&self) -> Constraint;

//     fn build<F>(
//         &self,
//         node: &RectNode,
//         tree: &Rectree,
//         set_translation: F,
//     ) -> Size
//     where
//         F: FnMut(NodeId, Vec2);
// }

// impl FixedArea {
//     fn layout(&self, constraint: Constraint) -> Size {
//         match (constraint.width, constraint.height) {
//             (None, None) => {
//                 // Square
//                 Size::splat(self.target_area.sqrt())
//             }
//             (None, Some(h)) => Size::new(self.target_area / h, h),
//             (Some(w), None) => Size::new(w, self.target_area / w),
//             (Some(w), Some(h)) => {
//                 if self.use_width {
//                     Size::new(w, self.target_area / w)
//                 } else {
//                     Size::new(self.target_area / h, h)
//                 }
//             }
//         }
//     }
// }

#[derive(Default)]
struct LayoutDemo {
    tree: Rectree,
    world: World,
}

impl LayoutDemo {
    fn new() -> Self {
        Self {
            tree: Rectree::new(),
            world: World::default(),
        }
    }

    fn add_area(
        &mut self,
        parent: Option<NodeId>,
        color: Color,
        add_content: impl FnOnce(&mut Self, NodeId) -> FixedArea,
    ) -> NodeId {
        let mut node = RectNode::new();
        if let Some(parent) = parent {
            node = node.with_parent(parent);
        }
        let id = self.tree.insert(node);

        let c = add_content(self, id);
        self.world.areas.insert(id, c);
        self.world.node_colors.insert(id, color);

        id
    }

    fn add_vertical(
        &mut self,
        parent: Option<NodeId>,
        color: Color,
        add_content: impl FnOnce(
            &mut Self,
            NodeId,
        ) -> VerticalCenteredList,
    ) -> NodeId {
        let mut node = RectNode::new();
        if let Some(parent) = parent {
            node = node.with_parent(parent);
        }
        let id = self.tree.insert(node);

        let c = add_content(self, id);
        self.world.verticals.insert(id, c);
        self.world.node_colors.insert(id, color);

        id
    }

    fn draw_tree(&self, scene: &mut Scene, transform: Affine) {
        // Start traversal from the root IDs provided by the tree.
        for root_id in self.tree.root_ids() {
            let mut stack = vec![*root_id];

            while let Some(node_id) = stack.pop() {
                // Get node from tree.
                let node = self.tree.get(&node_id);

                // Get world_translation.
                let world_pos = node.world_translation();

                // Reconstruct rect from world pos and size.
                let world_rect = Rect::from_origin_size(
                    world_pos.to_point(),
                    node.size,
                );

                // Fetch node color.
                let color = self
                    .world
                    .node_colors
                    .get(&node_id)
                    .cloned()
                    .unwrap_or(Color::WHITE);

                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    color,
                    None,
                    &world_rect,
                );

                scene.stroke(
                    &Stroke::new(2.0),
                    transform,
                    Color::from_rgb8(255, 255, 255),
                    None,
                    &world_rect,
                );

                // Origin markers.
                let origin = Circle::new(world_rect.origin(), 5.0);

                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    Color::from_rgb8(255, 50, 50),
                    None,
                    &origin,
                );

                // Traverse to children.
                for child_id in node.children().iter() {
                    stack.push(*child_id);
                }
            }
        }
    }
}

impl VelloDemo for LayoutDemo {
    fn window_title(&self) -> &'static str {
        "Layout Showcase"
    }
    fn initial_logical_size(&self) -> (f64, f64) {
        (800.0, 600.0)
    }

    fn rebuild_scene(
        &mut self,
        scene: &mut Scene,
        _scale_factor: f64,
    ) {
        // Create an oscillating translation vector.
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let mut ctx = LayoutCtx::new(&mut self.tree);

        for (i, (id, area)) in self.world.areas.iter_mut().enumerate()
        {
            let time = time + i as f64;
            let oscillation = (time.cos() + 1.0) * AREA;

            area.target_area = AREA + oscillation;
            ctx.schedule_relayout(*id);
        }

        // Perform layouting.
        ctx.layout(&self.world);

        self.draw_tree(scene, Affine::IDENTITY);
    }
}

pub trait SplatExt {
    fn splat(v: f64) -> Self;
}

impl SplatExt for Size {
    fn splat(v: f64) -> Self {
        Size::new(v, v)
    }
}
