use std::any::Any;

use hashbrown::HashMap;
use kurbo::{Affine, Rect, Size, Stroke, Vec2};
use rectree::layout::{
    Constraint, LayoutSolver, LayoutWorld, Positioner,
};
use rectree::node::RectNode;
use rectree::{NodeId, Rectree};
use vello::Scene;
use vello::peniko::Color;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut demo = LayoutDemo::new();

    // Create the Vertical Stack Container
    demo.add_widget(None, Color::TRANSPARENT, |demo, id| Vertical {
        spacing: 20.0,
        children: vec![
            // Block 1: Red
            demo.add_widget(
                Some(id),
                Color::from_rgb8(255, 100, 100),
                |_, _| FixedHeightRect { height: 100.0 },
            ),
            // Block 2: Green
            demo.add_widget(
                Some(id),
                Color::from_rgb8(100, 255, 100),
                |_, _| FixedHeightRect { height: 200.0 },
            ),
            // Block 3: Blue
            demo.add_widget(
                Some(id),
                Color::from_rgb8(100, 100, 255),
                |_, _| FixedHeightRect { height: 130.0 },
            ),
        ],
    });

    // Initial layout pass
    demo.tree.layout(&demo.world);

    let mut app = VelloWinitApp::new(demo);
    event_loop.run_app(&mut app).unwrap();
}

// Vertical Layout Widget
#[derive(Debug, Clone)]
struct Vertical {
    spacing: f64,
    children: Vec<NodeId>,
}

// Vertical Logic
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
        // Enforce parent's width and fixed height
        let width = node.parent_constraint().width.unwrap_or(200.0);
        Size::new(width, self.height)
    }
}

pub trait Widget: LayoutSolver + Any {}
impl<T> Widget for T where T: LayoutSolver + Any {}

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
        for root_id in self.tree.root_ids() {
            let mut stack = vec![*root_id];

            while let Some(node_id) = stack.pop() {
                let node = self.tree.get(&node_id);
                let world_pos = node.world_translation();
                let world_rect = Rect::from_origin_size(
                    world_pos.to_point(),
                    node.size(),
                );

                let color = self
                    .world
                    .node_colors
                    .get(&node_id)
                    .cloned()
                    .unwrap_or(Color::WHITE);

                // Only draw if not fully transparent
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
                        Color::WHITE,
                        None,
                        &world_rect,
                    );
                }

                for child_id in node.children().iter() {
                    stack.push(*child_id);
                }
            }
        }
    }
}

impl VelloDemo for LayoutDemo {
    fn window_title(&self) -> &'static str {
        "Vertical Layout Showcase"
    }
    fn initial_logical_size(&self) -> (f64, f64) {
        (600.0, 800.0)
    }

    fn rebuild_scene(
        &mut self,
        scene: &mut Scene,
        _scale_factor: f64,
    ) {
        self.tree.layout(&self.world);
        self.draw_tree(scene, Affine::IDENTITY);
    }
}
