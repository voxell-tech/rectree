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
use vello::peniko::color::palette::css;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut demo = LayoutDemo::new();
    let mut builder = demo.builder();

    let create_column = |b: &mut Builder| {
        Vertical::new(10.0).show(b, |b| {
            const WIDTH: f64 = 200.0;
            vec![
                FixedSizeWidget::new(Size::new(WIDTH, 40.0))
                    .with_color(css::RED)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 60.0))
                    .with_color(css::ORANGE)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 80.0))
                    .with_color(css::YELLOW)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 100.0))
                    .with_color(css::GREEN)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 80.0))
                    .with_color(css::BLUE)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 60.0))
                    .with_color(css::VIOLET)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 40.0))
                    .with_color(css::PURPLE)
                    .show(b),
            ]
        })
    };

    let root_id = FixedSizeWidget::new(builder.demo.window_size)
        .show_with_child(&mut builder, |b| {
            Padding::all(20.0).show(b, |b| {
                Vertical::new(20.0).show(b, |b| {
                    const HEIGHT: f64 = 60.0;
                    vec![
                        Horizontal::new(50.0).show(b, |b| {
                            vec![
                                create_column(b),
                                create_column(b),
                                create_column(b),
                            ]
                        }),
                        FixedSizeWidget::new(Size::new(50.0, HEIGHT))
                            .with_color(css::CYAN)
                            .show(b),
                        FixedSizeWidget::new(Size::new(
                            200.0, HEIGHT,
                        ))
                        .with_color(css::SALMON)
                        .show(b),
                        FixedSizeWidget::new(Size::new(
                            800.0, HEIGHT,
                        ))
                        .with_color(css::RED)
                        .show(b),
                    ]
                })
            });
        });

    // Store the root ID for future reference.
    demo.root_id = Some(root_id);

    // Initial layout.
    demo.tree.layout(&demo.world);

    let mut app = VelloWinitApp::new(demo);

    event_loop.run_app(&mut app).unwrap();
}

pub struct World {
    widgets: HashMap<NodeId, Box<dyn Widget>>,
}

impl World {
    fn new() -> Self {
        Self {
            widgets: HashMap::new(),
        }
    }
}

impl LayoutWorld for World {
    fn get_solver(&self, id: &NodeId) -> &dyn LayoutSolver {
        &**self.widgets.get(id).unwrap()
    }
}

pub trait Widget: LayoutSolver + Any {}

impl<T> Widget for T where T: LayoutSolver + Any {}

pub struct LayoutDemo {
    tree: Rectree,
    world: World,
    window_size: Size,
    root_id: Option<NodeId>,
}

pub struct Builder<'a> {
    pub demo: &'a mut LayoutDemo,
    pub parent_id: Option<NodeId>,
}

impl Builder<'_> {
    pub fn add_widget<W: Widget + 'static>(
        &mut self,
        add_content: impl FnOnce(&mut Builder) -> W,
    ) -> NodeId {
        let mut node = RectNode::new();
        if let Some(parent_id) = self.parent_id {
            node = node.with_parent(parent_id);
        }
        let id = self.demo.tree.insert(node);

        let w = Box::new(add_content(&mut Builder {
            demo: self.demo,
            parent_id: Some(id),
        }));
        self.demo.world.widgets.insert(id, w);

        id
    }
}

impl LayoutDemo {
    pub fn new() -> Self {
        Self {
            tree: Rectree::new(),
            world: World::new(),
            window_size: Size::new(800.0, 600.0),
            root_id: None,
        }
    }

    pub fn builder(&mut self) -> Builder<'_> {
        Builder {
            demo: self,
            parent_id: None,
        }
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

                // Hack to get the color of `FixedSizeWidget`.
                // In real world scenario, you would want to
                // implement a `draw` method for your `Widget` trait.
                if let Some(color) =
                    self.world.widgets.get(&node_id).and_then(
                        |widget| {
                            let widget: &dyn Any = widget.as_ref();
                            widget
                                .downcast_ref::<FixedSizeWidget>()
                                .map(|f| f.color)
                        },
                    )
                {
                    scene.fill(
                        vello::peniko::Fill::NonZero,
                        transform,
                        color,
                        None,
                        &world_rect,
                    );
                }

                scene.stroke(
                    &Stroke::new(2.0),
                    transform,
                    Color::WHITE,
                    None,
                    &world_rect,
                );

                // Origin markers.
                let origin = Circle::new(world_rect.origin(), 5.0);

                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    css::RED,
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

impl Default for LayoutDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl VelloDemo for LayoutDemo {
    fn window_title(&self) -> &'static str {
        "Layout Showcase"
    }

    fn initial_logical_size(&self) -> (f64, f64) {
        (self.window_size.width, self.window_size.height)
    }

    fn size_changed(&mut self, size: Size) {
        self.window_size = size;

        // Propagate size change to the root widget.
        let Some(root_id) = self.root_id else { return };

        let Some(widget) = self.world.widgets.get_mut(&root_id)
        else {
            return;
        };

        if let Some(fixed_widget) = (widget.as_mut() as &mut dyn Any)
            .downcast_mut::<FixedSizeWidget>()
        {
            fixed_widget.size = size;
            // Trigger relayout for the root.
            self.tree.schedule_relayout(root_id);
        }
    }

    fn rebuild_scene(
        &mut self,
        scene: &mut Scene,
        scale_factor: f64,
    ) {
        // Perform layouting.
        self.tree.layout(&self.world);

        self.draw_tree(scene, Affine::scale(scale_factor));
    }
}

// Below are some demo widgets to demonstrate how a UI library could
// potentially use `rectree` as a backend!

/// [`HorizontalWidget`] builder.
#[derive(Debug, Clone)]
pub struct Horizontal {
    pub spacing: f64,
}

impl Horizontal {
    pub fn new(spacing: f64) -> Self {
        Self { spacing }
    }
    pub fn show(
        self,
        builder: &mut Builder,
        add_content: impl FnOnce(&mut Builder) -> Vec<NodeId>,
    ) -> NodeId {
        builder.add_widget(|b| HorizontalWidget {
            style: self,
            children: add_content(b),
        })
    }
}

// Horizontal layout widget.
#[derive(Debug, Clone)]
pub struct HorizontalWidget {
    pub style: Horizontal,
    pub children: Vec<NodeId>,
}

impl LayoutSolver for HorizontalWidget {
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
        _node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let mut max_height = 0.0;
        let mut x_cursor = 0.0;

        for id in self.children.iter() {
            let child_node = tree.get(id);
            let child_size = child_node.size();

            positioner.set(*id, Vec2::new(x_cursor, 0.0));
            x_cursor += child_size.width + self.style.spacing;

            // Track the tallest child
            if child_size.height > max_height {
                max_height = child_size.height;
            }
        }
        // Remove the last added spacing
        if !self.children.is_empty() {
            x_cursor -= self.style.spacing;
        }

        Size::new(x_cursor, max_height)
    }
}

/// [`VerticalWidget`] builder.
#[derive(Debug, Clone)]
pub struct Vertical {
    pub spacing: f64,
}

impl Vertical {
    pub fn new(spacing: f64) -> Self {
        Self { spacing }
    }
    pub fn show(
        self,
        builder: &mut Builder,
        add_content: impl FnOnce(&mut Builder) -> Vec<NodeId>,
    ) -> NodeId {
        builder.add_widget(|b| VerticalWidget {
            style: self,
            children: add_content(b),
        })
    }
}

// Vertical layout widget.
#[derive(Debug, Clone)]
pub struct VerticalWidget {
    pub style: Vertical,
    pub children: Vec<NodeId>,
}

impl LayoutSolver for VerticalWidget {
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
        _node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let mut max_width = 0.0;
        let mut y_cursor = 0.0;

        for id in self.children.iter() {
            let child_node = tree.get(id);
            let child_size = child_node.size();

            positioner.set(*id, Vec2::new(0.0, y_cursor));

            y_cursor += child_size.height + self.style.spacing;
            // Track the widest child
            if child_size.width > max_width {
                max_width = child_size.width;
            }
        }
        // Remove the last added spacing
        if !self.children.is_empty() {
            y_cursor -= self.style.spacing;
        }

        Size::new(max_width, y_cursor)
    }
}

/// [`PaddingWidget`] builder.
#[derive(Debug, Clone, Copy)]
pub struct Padding {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl Padding {
    fn all(padding: f64) -> Self {
        Self {
            left: padding,
            right: padding,
            top: padding,
            bottom: padding,
        }
    }

    fn show(
        self,
        builder: &mut Builder,
        add_content: impl FnOnce(&mut Builder) -> NodeId,
    ) -> NodeId {
        builder.add_widget(|b| PaddingWidget {
            style: self,
            child: add_content(b),
        })
    }
}

/// A container widget that applies specific padding to each side.
#[derive(Debug)]
pub struct PaddingWidget {
    pub style: Padding,
    pub child: NodeId,
}

impl LayoutSolver for PaddingWidget {
    fn constraint(
        &self,
        parent_constraint: Constraint,
    ) -> Constraint {
        let Padding {
            left,
            right,
            top,
            bottom,
        } = self.style;

        Constraint {
            // Subtract horizontal padding from width
            width: parent_constraint
                .width
                .map(|w| (w - (left + right)).max(0.0)),
            // Subtract vertical padding from height
            height: parent_constraint
                .height
                .map(|h| (h - (top + bottom)).max(0.0)),
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
        let Padding {
            left,
            right,
            top,
            bottom,
        } = self.style;

        let child_node = tree.get(&self.child);
        let child_size = child_node.size();

        // Position the child with the specified padding offsets
        positioner.set(self.child, Vec2::new(left, top));

        Size::new(
            child_size.width + left + right,
            child_size.height + top + bottom,
        )
    }
}

/// A widget that forces a specific size that ignore parent constraints.
#[derive(Debug, Clone)]
pub struct FixedSizeWidget {
    pub size: Size,
    pub color: Color,
}

impl LayoutSolver for FixedSizeWidget {
    fn constraint(&self, _parent: Constraint) -> Constraint {
        // Fixed size yield fixed contraint.
        Constraint::fixed(self.size.width, self.size.height)
    }

    fn build(
        &self,
        _node: &RectNode,
        _tree: &Rectree,
        _positioner: &mut Positioner,
    ) -> Size {
        self.size
    }
}

impl FixedSizeWidget {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            color: Color::TRANSPARENT,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn show(self, b: &mut Builder) -> NodeId {
        b.add_widget(|_| self)
    }

    pub fn show_with_child(
        self,
        b: &mut Builder,
        add_content: impl FnOnce(&mut Builder),
    ) -> NodeId {
        b.add_widget(|b| {
            add_content(b);
            self
        })
    }
}
