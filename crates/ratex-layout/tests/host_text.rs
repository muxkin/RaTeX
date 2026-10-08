use std::cell::RefCell;
use std::process::Command;

use ratex_layout::layout_box::BoxContent;
use ratex_layout::{layout, to_display_list, LayoutBox, LayoutOptions, TextLayout};
use ratex_parser::parse_node::Mode;
use ratex_parser::{parser::parse, ParseNode};
use ratex_types::{color::Color, math_style::MathStyle, PathCommand};

#[derive(Debug)]
struct Run {
    text: String,
    weight: Option<u16>,
    italic: Option<bool>,
    color: Color,
    style: MathStyle,
}

#[derive(Default)]
struct Host {
    runs: RefCell<Vec<Run>>,
}

fn literal(nodes: &[ParseNode]) -> Option<String> {
    nodes
        .iter()
        .map(|node| match node {
            ParseNode::TextOrd {
                text,
                mode: Mode::Text,
                ..
            }
            | ParseNode::MathOrd {
                text,
                mode: Mode::Text,
                ..
            } => Some(text.clone()),
            ParseNode::OrdGroup {
                body,
                mode: Mode::Text,
                ..
            } => literal(body),
            ParseNode::SpacingNode {
                text,
                mode: Mode::Text,
                ..
            } if matches!(
                text.as_str(),
                " " | "~" | "\\ " | "\\space" | "\\nobreakspace"
            ) =>
            {
                Some(" ".into())
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.concat())
}

impl TextLayout for Host {
    fn layout(&self, body: &[ParseNode], options: &LayoutOptions) -> Option<LayoutBox> {
        let text = literal(body)?;
        if text.is_empty() {
            return None;
        }
        let width = text.chars().count() as f64 * 1.5;
        self.runs.borrow_mut().push(Run {
            text,
            weight: options.text_weight,
            italic: options.text_italic,
            color: options.color,
            style: options.style,
        });
        Some(LayoutBox {
            width,
            height: 0.8,
            depth: 0.2,
            content: BoxContent::SvgPath {
                commands: vec![
                    PathCommand::MoveTo { x: 0.0, y: -0.8 },
                    PathCommand::LineTo { x: width, y: -0.8 },
                    PathCommand::LineTo { x: width, y: 0.2 },
                    PathCommand::LineTo { x: 0.0, y: 0.2 },
                    PathCommand::Close,
                ],
                fill: true,
            },
            color: options.color,
        })
    }
}

#[test]
fn adjacent_unicode_text_is_offered_as_one_measured_run() {
    let host = Host::default();
    let options = LayoutOptions {
        text_layout: Some(&host),
        ..Default::default()
    };
    let ast = parse(r"\text{你好 世界! fi}").unwrap();
    let result = layout(&ast, &options);
    let runs = host.runs.borrow();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "你好 世界! fi");
    assert_eq!(result.width, 13.5);
    assert_eq!(result.height, 0.8);
    assert_eq!(result.depth, 0.2);
    // Host outlines use the existing protocol and round-trip through JSON.
    let display = to_display_list(&result);
    let json = serde_json::to_string(&display).unwrap();
    let round_trip: ratex_types::DisplayList = serde_json::from_str(&json).unwrap();
    assert_eq!(display.items, round_trip.items);
}

#[test]
fn measured_text_controls_script_width_and_receives_script_style() {
    let host = Host::default();
    let options = LayoutOptions {
        text_layout: Some(&host),
        ..Default::default()
    };
    let ast = parse(r"E^{\text{你好}}_{\text{世界！}}+x").unwrap();
    let result = layout(&ast, &options);
    let BoxContent::HBox(children) = &result.content else {
        panic!("expected expression")
    };
    let BoxContent::SupSub {
        sup: Some(sup),
        sub: Some(sub),
        sup_scale,
        sub_scale,
        ..
    } = &children[0].content
    else {
        panic!("expected scripts")
    };
    assert_eq!(sup.width, 3.0);
    assert_eq!(sub.width, 4.5);
    assert!((*sup_scale - 0.7).abs() < 1e-12);
    assert!((*sub_scale - 0.7).abs() < 1e-12);
    assert!(children[0].width >= sub.width * sub_scale);
    let runs = host.runs.borrow();
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().all(|run| run.style.is_tight()));
}

#[test]
fn nested_text_commands_preserve_face_and_color() {
    let host = Host::default();
    let options = LayoutOptions {
        text_layout: Some(&host),
        ..Default::default()
    };
    let ast = parse(r"\textbf{A\textit{B}\textnormal{C}}+\textcolor{red}{\text{D}}").unwrap();
    layout(&ast, &options);
    let runs = host.runs.borrow();
    for (text, weight, italic) in [
        ("A", Some(700), None),
        ("B", Some(700), Some(true)),
        ("C", Some(400), Some(false)),
    ] {
        let run = runs.iter().find(|run| run.text == text).unwrap();
        assert_eq!((run.weight, run.italic), (weight, italic));
    }
    assert_eq!(
        runs.iter().find(|run| run.text == "D").unwrap().color,
        Color::rgb(1.0, 0.0, 0.0)
    );
}

#[test]
fn chemistry_proof_cells_and_math_alphabets_keep_distinct_paths() {
    let host = Host::default();
    let options = LayoutOptions {
        text_layout: Some(&host),
        ..Default::default()
    };
    for input in [
        r"\ce{A ->[\text{催化}] B}",
        r"\begin{prooftree}\AxiomC{假设}\UnaryInfC{结论}\end{prooftree}",
    ] {
        let result = layout(&parse(input).unwrap(), &options);
        assert!(result.width.is_finite() && result.width > 0.0);
    }
    let runs = host.runs.borrow();
    for text in ["催化", "假设", "结论"] {
        assert!(runs.iter().any(|run| run.text == text), "missing {text}");
    }
    drop(runs);
    host.runs.borrow_mut().clear();
    layout(
        &parse(r"\mathbf{x}+\mathcal{A}+\boldsymbol{\alpha}").unwrap(),
        &options,
    );
    assert!(host.runs.borrow().is_empty());
}

struct Decline;
impl TextLayout for Decline {
    fn layout(&self, _: &[ParseNode], _: &LayoutOptions) -> Option<LayoutBox> {
        None
    }
}

#[test]
fn declining_host_preserves_bundled_font_display_list() {
    let host = Decline;
    for input in [
        r"x+\frac{a}{b}",
        r"\text{office test}",
        r"\textbf{A\textit{B}}",
        r"E^{\text{upper}}_{\text{lower}}",
        r"\ce{A\bond{~--}B}",
        r"\begin{prooftree}\AxiomC{P}\UnaryInfC{Q}\end{prooftree}",
        r"\left(\text{test}\right)",
    ] {
        let ast = parse(input).unwrap();
        let plain = to_display_list(&layout(&ast, &LayoutOptions::default()));
        let hooked = to_display_list(&layout(
            &ast,
            &LayoutOptions {
                text_layout: Some(&host),
                ..Default::default()
            },
        ));
        assert_eq!(
            serde_json::to_value(plain).unwrap(),
            serde_json::to_value(hooked).unwrap(),
            "{input}"
        );
    }
}

#[test]
fn host_text_depth_boundary_on_small_release_stack() {
    const CHILD: &str = "RATEX_HOST_TEXT_DEPTH_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "host_text_depth_boundary_on_small_release_stack"])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let check = || {
        let host = Host::default();
        let options = LayoutOptions {
            text_layout: Some(&host),
            ..Default::default()
        };
        let accepted = format!("{}你好{}", "{".repeat(32), "}".repeat(32));
        let result = to_display_list(&layout(&parse(&accepted).unwrap(), &options));
        assert!(result.width.is_finite());
        for depth in [33, 300] {
            let rejected = format!("{}你好{}", "{".repeat(depth), "}".repeat(depth));
            assert!(parse(&rejected)
                .unwrap_err()
                .to_string()
                .contains("Recursion limit exceeded"));
        }
    };
    // Debug parser frames exceed the production stack budget.
    let stack = if cfg!(debug_assertions) {
        8 * 1024 * 1024
    } else {
        512 * 1024
    };
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}
