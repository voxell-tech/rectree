use rectree::kurbo::Vec2;
use rectree::{RectNode, Rectree};

fn main() {
    let mut tree = Rectree::new();

    // Insert root rect.
    let root_id = tree
        .insert_node(RectNode::from_size(Vec2::new(100.0, 100.0)));
    let child_id = tree.insert_node(
        RectNode::from_translation_size(
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 10.0),
        )
        .with_parent(root_id),
    );
    tree.insert_node(
        RectNode::from_translation_size(
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 10.0),
        )
        .with_parent(child_id),
    );

    tree.update_translations();

    let mut stack = tree.root_ids().iter().collect::<Vec<_>>();

    while let Some(id) = stack.pop() {
        if let Some(node) = tree.get_node(id) {
            for _ in 0..node.depth() {
                print!("  ");
            }

            println!(
                "translation: {}, size: {}",
                node.world_translation(),
                &*node.size
            );

            stack.extend(node.children().iter());
        }
    }
}
