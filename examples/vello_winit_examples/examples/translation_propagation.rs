use hashbrown::HashMap;
use rectree::node::RectNode;
use rectree::{NodeId, Rectree};
use vello::Scene;
use vello::kurbo::{Affine, Circle, Rect, Stroke, Vec2};
use vello::peniko::Color;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

struct TranslationDemo {
    tree: Rectree,
    root_id: NodeId,
    node_colors: HashMap<NodeId, Color>,
}

impl TranslationDemo {
    fn new() -> Self {
        let mut tree = Rectree::new();
        let mut node_colors = HashMap::new();

        // Insert root node (Blue).
        let root_id = tree.insert_node(RectNode::from_rect(
            Rect::new(0.0, 0.0, 200.0, 200.0),
        ));
        node_colors.insert(root_id, Color::from_rgb8(80, 120, 200));

        // Insert child node (Green) translated relative to root.
        let child_id = tree.insert_node(
            RectNode::from_rect(Rect::new(0.0, 0.0, 80.0, 80.0))
                .with_parent(root_id)
                .with_translation(Vec2::new(40.0, 40.0)),
        );
        node_colors.insert(child_id, Color::from_rgb8(120, 200, 120));

        // Insert grandchild node (Orange) translated relative to the child.
        let grandchild_id = tree.insert_node(
            RectNode::from_rect(Rect::new(0.0, 0.0, 30.0, 30.0))
                .with_parent(child_id)
                .with_translation(Vec2::new(10.0, 10.0)),
        );
        node_colors
            .insert(grandchild_id, Color::from_rgb8(200, 120, 80));

        tree.update_translations();

        Self {
            tree,
            root_id,
            node_colors,
        }
    }

    fn draw_tree(&self, scene: &mut Scene, transform: Affine) {
        // Start traversal from the root IDs provided by the tree.
        for root_id in self.tree.root_ids() {
            let mut stack = vec![*root_id];

            while let Some(node_id) = stack.pop() {
                // Get node from tree.
                if let Some(node) = self.tree.get_node(&node_id) {
                    // Get world_translation.
                    let world_pos = node.world_translation();

                    // Reconstruct rect from world pos and size.
                    let world_rect = Rect::from_origin_size(
                        world_pos.to_point(),
                        node.size,
                    );

                    // Fetch node color.
                    let color = self
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
                    let origin =
                        Circle::new(world_rect.origin(), 5.0);

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
}

impl VelloDemo for TranslationDemo {
    fn window_title(&self) -> &'static str {
        "Rectree Translation Showcase"
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
        let oscillation =
            Vec2::new(time.cos() * 60.0, time.sin() * 60.0);

        // Modify ONLY the parent's local translation.
        self.tree.with_node_mut(&self.root_id, |node| {
            *node.local_translation = oscillation;
        });

        // Recalculate world positions.
        self.tree.update_translations();

        let transform = Affine::translate((150.0, 150.0));
        self.draw_tree(scene, transform);
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = VelloWinitApp::new(TranslationDemo::new());

    event_loop.run_app(&mut app).unwrap();
}
