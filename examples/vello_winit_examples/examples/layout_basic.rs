use std::any::Any;

use hashbrown::HashMap;
use kurbo::{Affine, Circle, Rect, Size, Stroke, Vec2};
use rectree::layout::{
    Constraint, LayoutSolver, LayoutWorld, Positioner,
};
use rectree::node::RectNode;
use rectree::{NodeId, Rectree};
use vello::Scene;
use vello::peniko::Color;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

const AREA: f64 = 2500.0;
const WINDOW_WIDTH: f64 = 800.0;
const WINDOW_HEIGHT: f64 = 600.0;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut demo = LayoutDemo::new();
    // Root widget that aligns its content to the center of the window.
    demo.add_widget(None, Color::TRANSPARENT, |demo, align_root| {
        // Create a horizontal stack container.
        let content = demo.add_widget(
            Some(align_root),
            Color::TRANSPARENT,
            |demo, root_id| Horizontal {
                spacing: 20.0,
                children: vec![
                    demo.add_widget(
                        Some(root_id),
                        Color::from_rgb8(200, 200, 10),
                        |demo, id| VerticalCenteredList {
                            padding: 20.0,
                            children: (0..5)
                                .map(|_i| {
                                    let area = FixedArea {
                                        use_width: false,
                                        target_area: AREA,
                                    };

                                    demo.add_widget(
                                        Some(id),
                                        Color::from_rgb8(
                                            10, 200, 200,
                                        ),
                                        |_, _| area,
                                    )
                                })
                                .collect(),
                        },
                    ),
                    demo.add_widget(
                        Some(root_id),
                        // Visualize padding container with white background
                        Color::WHITE,
                        // Create a vertical stack of fixed height rectangles
                        |demo, id| {
                            let child = demo.add_widget(
                                Some(id),
                                Color::TRANSPARENT,
                                |demo, id| Vertical {
                                    spacing: 20.0,
                                    children: vec![
                                        demo.add_widget(
                                            Some(id),
                                            Color::from_rgb8(
                                                255, 100, 100,
                                            ),
                                            |_, _| FixedHeightRect {
                                                height: 100.0,
                                            },
                                        ),
                                        // Margin example using Padding widget
                                        demo.add_widget(
                                            Some(id),
                                            Color::TRANSPARENT,
                                            |demo, parent_id| Padding {
                                                pad_x: 0.0,
                                                pad_y: 30.0,
                                                child: demo.add_widget(
                                                    Some(parent_id),
                                                    Color::from_rgb8(100, 255, 100),
                                                    |_, _| FixedHeightRect {
                                                        height: 200.0,
                                                    },
                                                ),
                                            },
                                        ),
                                        demo.add_widget(
                                            Some(id),
                                            Color::from_rgb8(
                                                100, 100, 255,
                                            ),
                                            |_, _| FixedHeightRect {
                                                height: 130.0,
                                            },
                                        ),
                                    ],
                                },
                            );
                            // Wrap the vertical stack in a padding container
                            Padding {
                                pad_x: 20.0,
                                pad_y: 20.0,
                                child,
                            }
                        },
                    ),
                ],
            },
        );
        // Align the content in the center of the window
        Align {
            align_x: 0.5,
            align_y: 0.5,
            child: content,
        }
    });

    // Initial layout.
    demo.tree.layout(&demo.world);

    let mut app = VelloWinitApp::new(demo);

    event_loop.run_app(&mut app).unwrap();
}

/// Aligns a child node within the available space.
#[derive(Debug, Clone)]
struct Align {
    /// Horizontal alignment: 0.0 = Left, 0.5 = Center, 1.0 = Right.
    align_x: f64,
    /// Vertical alignment: 0.0 = Top, 0.5 = Center, 1.0 = Bottom.
    align_y: f64,
    /// The target node to be aligned.
    child: NodeId,
}

impl LayoutSolver for Align {
    fn constraint(&self, _parent: Constraint) -> Constraint {
        Constraint {
            width: None,
            height: None,
        }
    }

    fn build(
        &self,
        node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let child_node = tree.get(&self.child);
        let child_size = child_node.size();

        let width = child_size.width.max(1.0);
        let height = child_size.height.max(1.0);

        let container_width =
            node.parent_constraint().width.unwrap_or(WINDOW_WIDTH);
        let container_height =
            node.parent_constraint().height.unwrap_or(WINDOW_HEIGHT);

        let surplus_width = (container_width - width).max(0.0);
        let surplus_height = (container_height - height).max(0.0);

        let offset_x = surplus_width * self.align_x.clamp(0.0, 1.0);
        let offset_y = surplus_height * self.align_y.clamp(0.0, 1.0);

        positioner.set(self.child, Vec2::new(offset_x, offset_y));

        Size::new(container_width, container_height)
    }
}

/// A container widget that applies a fixed amount of padding around its single child.
/// This acts as a frame that consuming space from the parent before delegating to the child.
#[derive(Debug, Clone)]
struct Padding {
    /// The horizontal padding to apply on both sides.
    pad_x: f64,
    /// The vertical padding to apply on top and bottom.
    pad_y: f64,
    /// The unique identifier of the child node wrapped by this padding.
    child: NodeId,
}

impl LayoutSolver for Padding {
    fn constraint(
        &self,
        parent_constraint: Constraint,
    ) -> Constraint {
        Constraint {
            width: parent_constraint
                .width
                .map(|w| (w - self.pad_x * 2.0).max(0.0)),
            height: parent_constraint
                .height
                .map(|h| (h - self.pad_y * 2.0).max(0.0)),
        }
    }
    /// Determines the final size and position of the padding widget and its child.
    ///
    /// Retrieves the child's final calculated size.
    /// Offsets the child's position by the padding amount.
    /// Returns the total size of this widget,
    /// which includes the child's size plus the padding on all sides.
    fn build(
        &self,
        _node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let child_node = tree.get(&self.child);
        let child_size = child_node.size();

        positioner.set(self.child, Vec2::new(self.pad_x, self.pad_y));

        Size::new(
            child_size.width + self.pad_x * 2.0,
            child_size.height + self.pad_y * 2.0,
        )
    }
}

// Horizontal layout widget
#[derive(Debug, Clone)]
struct Horizontal {
    spacing: f64,
    children: Vec<NodeId>,
}
// Horizontal logic
// Enforce the parent's height on children
// Relax the width constraint so children can stack infinitely
impl LayoutSolver for Horizontal {
    fn constraint(
        &self,
        parent_constraint: Constraint,
    ) -> Constraint {
        Constraint {
            width: None,
            height: parent_constraint.height,
        }
    }

    fn build(
        &self,
        node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let height = node.parent_constraint().height.unwrap_or(0.0);

        let mut x_cursor = 0.0;

        for id in self.children.iter() {
            let child_node = tree.get(id);
            let child_size = child_node.size();

            positioner.set(*id, Vec2::new(x_cursor, 0.0));

            x_cursor += child_size.width + self.spacing;
        }
        // Remove the last added spacing
        if !self.children.is_empty() {
            x_cursor -= self.spacing;
        }

        Size::new(x_cursor, height)
    }
}

// Vertical layout widget
#[derive(Debug, Clone)]
struct Vertical {
    spacing: f64,
    children: Vec<NodeId>,
}
// Vertical logic
// Enforce the parent's width on children
// Relax the height constraint so children can stack infinitely
impl LayoutSolver for Vertical {
    fn constraint(
        &self,
        parent_constraint: Constraint,
    ) -> Constraint {
        Constraint {
            width: parent_constraint.width,
            height: None,
        }
    }

    fn build(
        &self,
        node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let width = node.parent_constraint().width.unwrap_or(200.0);

        let mut y_cursor = 0.0;

        for id in self.children.iter() {
            let child_node = tree.get(id);
            let child_size = child_node.size();

            positioner.set(*id, Vec2::new(0.0, y_cursor));

            y_cursor += child_size.height + self.spacing;
        }
        // Remove the last added spacing
        if !self.children.is_empty() {
            y_cursor -= self.spacing;
        }

        Size::new(width, y_cursor)
    }
}

#[derive(Debug, Clone, Copy)]
struct FixedHeightRect {
    height: f64,
}

impl LayoutSolver for FixedHeightRect {
    fn build(
        &self,
        node: &RectNode,
        _: &Rectree,
        _: &mut Positioner,
    ) -> Size {
        let width = node.parent_constraint().width.unwrap_or(200.0);
        Size::new(width, self.height)
    }
}

struct World {
    widgets: HashMap<NodeId, Box<dyn Widget>>,
    node_colors: HashMap<NodeId, Color>,
}

impl World {
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            node_colors: HashMap::new(),
        }
    }
}

impl LayoutWorld for World {
    fn get_solver(&self, id: &NodeId) -> &dyn LayoutSolver {
        &**self.widgets.get(id).unwrap()
    }
}

#[derive(Debug, Clone)]
struct VerticalCenteredList {
    padding: f64,
    // In real world scenario, this should just store the ids
    // mapping to an arena of widgets.
    children: Vec<NodeId>,
}

impl LayoutSolver for VerticalCenteredList {
    fn build(
        &self,
        node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let width =
            node.parent_constraint().width.unwrap_or_else(|| {
                let mut max_width = 0.0;

                for id in self.children.iter() {
                    let node = tree.get(id);
                    max_width = node.size().width.max(max_width);
                }

                max_width
            }) + self.padding * 2.0;

        let mut height = self.padding;
        for id in self.children.iter() {
            let node = tree.get(id);
            let size = node.size();
            let remainder = width - size.width;

            let x = remainder * 0.5;
            let y = height;
            positioner.set(*id, Vec2::new(x, y));

            height += size.height + self.padding;
        }

        Size::new(width, height)
    }
}

#[derive(Debug, Clone, Copy)]
struct FixedArea {
    /// Use width if constrained on both axis.
    /// (Acts like a text widget.)
    pub use_width: bool,
    pub target_area: f64,
}

impl LayoutSolver for FixedArea {
    fn build(
        &self,
        node: &RectNode,
        _: &Rectree,
        _: &mut Positioner,
    ) -> Size {
        let constraint = node.parent_constraint();
        match (constraint.width, constraint.height) {
            (None, None) => {
                // Square
                Size::splat(self.target_area.sqrt())
            }
            (None, Some(h)) => Size::new(self.target_area / h, h),
            (Some(w), None) => Size::new(w, self.target_area / w),
            (Some(w), Some(h)) => {
                if self.use_width {
                    Size::new(w, self.target_area / w)
                } else {
                    Size::new(self.target_area / h, h)
                }
            }
        }
    }
}

pub trait Widget: LayoutSolver + Any {}

impl<T> Widget for T where T: LayoutSolver + Any {}

struct LayoutDemo {
    tree: Rectree,
    world: World,
}

impl LayoutDemo {
    fn new() -> Self {
        Self {
            tree: Rectree::new(),
            world: World::new(),
        }
    }

    fn add_widget<W>(
        &mut self,
        parent: Option<NodeId>,
        color: Color,
        add_content: impl FnOnce(&mut Self, NodeId) -> W,
    ) -> NodeId
    where
        W: Widget + 'static,
    {
        let mut node = RectNode::new();
        if let Some(parent) = parent {
            node = node.with_parent(parent);
        }
        let id = self.tree.insert(node);

        let w = Box::new(add_content(self, id));
        self.world.widgets.insert(id, w);
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
                    node.size(),
                );

                // Fetch node color.
                let color = self
                    .world
                    .node_colors
                    .get(&node_id)
                    .cloned()
                    .unwrap_or(Color::WHITE);

                if color.components[3] > 0.0 {
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
                }

                // Origin markers.
                if color.components[3] > 0.0 {
                    let origin =
                        Circle::new(world_rect.origin(), 5.0);

                    scene.fill(
                        vello::peniko::Fill::NonZero,
                        transform,
                        Color::from_rgb8(255, 50, 50),
                        None,
                        &origin,
                    );
                }

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
        (WINDOW_WIDTH, WINDOW_HEIGHT)
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

        for (i, (id, widget)) in
            self.world.widgets.iter_mut().enumerate()
        {
            let widget = widget.as_mut() as &mut dyn Any;
            if let Some(area) = widget.downcast_mut::<FixedArea>() {
                let time = time + i as f64;
                let oscillation = (time.cos() + 1.0) * AREA;
                area.target_area = AREA + oscillation;
                self.tree.schedule_relayout(*id);
            }
        }

        // Perform layouting.
        self.tree.layout(&self.world);

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
