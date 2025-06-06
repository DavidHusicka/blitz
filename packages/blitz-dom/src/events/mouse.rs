use blitz_traits::{HitResult, MouseEventButtons, navigation::NavigationOptions};
use markup5ever::local_name;

use crate::{BaseDocument, Node, node::NodeSpecificData, util::resolve_url};

fn parent_hit(node: &Node, x: f32, y: f32) -> Option<HitResult> {
    node.layout_parent.get().map(|parent_id| HitResult {
        node_id: parent_id,
        x: x + node.final_layout.location.x,
        y: y + node.final_layout.location.y,
    })
}

pub(crate) fn handle_mousemove(
    doc: &mut BaseDocument,
    target: usize,
    x: f32,
    y: f32,
    buttons: MouseEventButtons,
) -> bool {
    let Some(hit) = doc.hit(x, y) else {
        return false;
    };

    if hit.node_id != target {
        return false;
    }

    let node = &mut doc.nodes[target];
    let Some(el) = node.data.downcast_element_mut() else {
        return false;
    };

    let disabled = el.attr(local_name!("disabled")).is_some();
    if disabled {
        return false;
    }

    if let NodeSpecificData::TextInput(ref mut text_input_data) = el.node_specific_data {
        if buttons == MouseEventButtons::None {
            return false;
        }

        let content_box_offset = taffy::Point {
            x: node.final_layout.padding.left + node.final_layout.border.left,
            y: node.final_layout.padding.top + node.final_layout.border.top,
        };

        let x = (hit.x - content_box_offset.x) as f64 * doc.viewport.scale_f64();
        let y = (hit.y - content_box_offset.y) as f64 * doc.viewport.scale_f64();

        text_input_data
            .editor
            .driver(&mut doc.font_ctx, &mut doc.layout_ctx)
            .extend_selection_to_point(x as f32, y as f32);

        return true;
    }

    false
}

pub(crate) fn handle_mousedown(doc: &mut BaseDocument, target: usize, x: f32, y: f32) {
    let Some(hit) = doc.hit(x, y) else {
        return;
    };
    if hit.node_id != target {
        return;
    }

    let node = &mut doc.nodes[target];
    let Some(el) = node.data.downcast_element_mut() else {
        return;
    };

    let disabled = el.attr(local_name!("disabled")).is_some();
    if disabled {
        return;
    }

    if let NodeSpecificData::TextInput(ref mut text_input_data) = el.node_specific_data {
        let content_box_offset = taffy::Point {
            x: node.final_layout.padding.left + node.final_layout.border.left,
            y: node.final_layout.padding.top + node.final_layout.border.top,
        };
        let x = (hit.x - content_box_offset.x) as f64 * doc.viewport.scale_f64();
        let y = (hit.y - content_box_offset.y) as f64 * doc.viewport.scale_f64();

        text_input_data
            .editor
            .driver(&mut doc.font_ctx, &mut doc.layout_ctx)
            .move_to_point(x as f32, y as f32);

        doc.set_focus_to(hit.node_id);
    }
}

pub(crate) fn handle_click(doc: &mut BaseDocument, _target: usize, x: f32, y: f32) {
    let mut maybe_hit = doc.hit(x, y);

    while let Some(hit) = maybe_hit {
        let node_id = hit.node_id;
        let maybe_element = {
            let node = &mut doc.nodes[node_id];
            node.data.downcast_element_mut()
        };

        let Some(el) = maybe_element else {
            maybe_hit = parent_hit(&doc.nodes[node_id], x, y);
            continue;
        };

        let disabled = el.attr(local_name!("disabled")).is_some();
        if disabled {
            return;
        }

        if let NodeSpecificData::TextInput(_) = el.node_specific_data {
            return;
        } else if el.name.local == local_name!("input")
            && matches!(el.attr(local_name!("type")), Some("checkbox"))
        {
            BaseDocument::toggle_checkbox(el);
            doc.set_focus_to(node_id);
            return;
        } else if el.name.local == local_name!("input")
            && matches!(el.attr(local_name!("type")), Some("radio"))
        {
            let radio_set = el.attr(local_name!("name")).unwrap().to_string();
            BaseDocument::toggle_radio(doc, radio_set, node_id);
            BaseDocument::set_focus_to(doc, node_id);
            return;
        }
        // Clicking labels triggers click, and possibly input event, of associated input
        else if el.name.local == local_name!("label") {
            if let Some(target_node_id) = doc
                .label_bound_input_elements(node_id)
                .first()
                .map(|n| n.id)
            {
                let target_node = doc.get_node_mut(target_node_id).unwrap();
                if let Some(target_element) = target_node.element_data_mut() {
                    BaseDocument::toggle_checkbox(target_element);
                }
                doc.set_focus_to(node_id);
                return;
            }
        } else if el.name.local == local_name!("a") {
            if let Some(href) = el.attr(local_name!("href")) {
                if let Some(url) = resolve_url(&doc.base_url, href) {
                    doc.navigation_provider.navigate_to(NavigationOptions::new(
                        url,
                        String::from("text/plain"),
                        doc.id(),
                    ));
                } else {
                    println!(
                        "{href} is not parseable as a url. : {base_url:?}",
                        base_url = doc.base_url
                    )
                }
                return;
            } else {
                println!("Clicked link without href: {:?}", el.attrs());
            }
        } else if el.name.local == local_name!("input")
            && el.attr(local_name!("type")) == Some("submit")
            || el.name.local == local_name!("button")
        {
            if let Some(form_owner) = doc.controls_to_form.get(&node_id) {
                doc.submit_form(*form_owner, node_id);
            }
        }

        // No match. Recurse up to parent.
        maybe_hit = parent_hit(&doc.nodes[node_id], x, y)
    }

    // If nothing is matched then clear focus
    doc.clear_focus();
}
