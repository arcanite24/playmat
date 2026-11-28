//! The bits of Jsoup the MTGTop8 page needs: the title, the first `div` with a class, and an element's text with
//! Jsoup's whitespace rules (`Element.text()`, `Element.ownText()`).

use ego_tree::NodeRef;
use scraper::{Html, Node};

pub struct Page {
    html: Html,
}

/// Jsoup's block tags (`Tag.isBlock`)
const BLOCK: &[&str] = &[
    "html",
    "head",
    "body",
    "frameset",
    "script",
    "noscript",
    "style",
    "meta",
    "link",
    "title",
    "frame",
    "noframes",
    "section",
    "nav",
    "aside",
    "hgroup",
    "header",
    "footer",
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "pre",
    "div",
    "blockquote",
    "hr",
    "address",
    "figure",
    "figcaption",
    "form",
    "fieldset",
    "ins",
    "del",
    "dl",
    "dt",
    "dd",
    "li",
    "table",
    "caption",
    "thead",
    "tfoot",
    "tbody",
    "colgroup",
    "col",
    "tr",
    "th",
    "td",
    "video",
    "audio",
    "canvas",
    "details",
    "menu",
    "plaintext",
    "template",
    "article",
    "main",
    "svg",
    "math",
    "center",
    "dir",
    "applet",
    "marquee",
    "listing",
];

/// block tags Jsoup formats inline (`Tag.formatAsBlock` false)
const FORMAT_INLINE: &[&str] = &[
    "title", "a", "p", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "address", "li", "th", "td", "script", "style",
    "ins", "del", "s", "button",
];

const PRESERVE_WHITESPACE: &[&str] = &["pre", "plaintext", "textarea", "listing"];

fn name<'a>(node: &NodeRef<'a, Node>) -> Option<&'a str> {
    node.value().as_element().map(|e| e.name())
}

fn is_block(node: &NodeRef<Node>) -> bool {
    name(node).is_some_and(|n| BLOCK.contains(&n))
}

fn formats_as_block(node: &NodeRef<Node>) -> bool {
    name(node).is_some_and(|n| BLOCK.contains(&n) && !FORMAT_INLINE.contains(&n))
}

fn last_is_space(accum: &str) -> bool {
    accum.ends_with(' ')
}

/// `TextNode` text into `accum`, as `Element.appendNormalisedText` does
fn append_text(accum: &mut String, node: &NodeRef<Node>, text: &str) {
    let mut parent = node.parent();
    for _ in 0..6 {
        let Some(at) = parent else { break };
        if name(&at).is_some_and(|n| PRESERVE_WHITESPACE.contains(&n)) {
            accum.push_str(text);
            return;
        }
        parent = at.parent();
    }
    let strip_leading = last_is_space(accum);
    let (mut last_white, mut reached) = (false, false);
    for c in text.chars() {
        if matches!(c, ' ' | '\t' | '\n' | '\x0C' | '\r' | '\u{a0}') {
            if (strip_leading && !reached) || last_white {
                continue;
            }
            accum.push(' ');
            last_white = true;
        } else if c != '\u{200b}' && c != '\u{ad}' {
            accum.push(c);
            last_white = false;
            reached = true;
        }
    }
}

fn java_trim(text: &str) -> String {
    super::java_trim(text).to_string()
}

pub type Element<'a> = NodeRef<'a, Node>;

impl Page {
    pub fn parse(page: &str) -> Page {
        Page { html: Html::parse_document(page) }
    }

    /// `Document.title()`
    pub fn title(&self) -> String {
        let head = self.html.tree.root().descendants().find(|n| name(n) == Some("head"));
        let title = head.and_then(|h| h.descendants().skip(1).find(|n| name(n) == Some("title")));
        let Some(title) = title else { return String::new() };
        let mut normalised = String::new();
        for c in text(&title).chars() {
            if matches!(c, ' ' | '\t' | '\n' | '\x0C' | '\r' | '\u{a0}') {
                if !normalised.ends_with(' ') {
                    normalised.push(' ');
                }
            } else if c != '\u{200b}' && c != '\u{ad}' {
                normalised.push(c);
            }
        }
        java_trim(&normalised)
    }

    /// `selectFirst("div.<class>")`
    pub fn first_div(&self, class: &str) -> Option<Element<'_>> {
        self.html.tree.root().descendants().find(|n| {
            n.value().as_element().is_some_and(|e| {
                e.name() == "div"
                    && e.attr("class").is_some_and(|classes| {
                        classes.split([' ', '\t', '\n', '\x0C', '\r']).any(|c| c.eq_ignore_ascii_case(class))
                    })
            })
        })
    }
}

/// `Element.text()`
pub fn text(element: &Element) -> String {
    let mut accum = String::new();
    walk(element, &mut accum);
    java_trim(&accum)
}

fn walk(node: &NodeRef<Node>, accum: &mut String) {
    match node.value() {
        Node::Text(text) => append_text(accum, node, text),
        Node::Element(e) => {
            if !accum.is_empty() && (is_block(node) || e.name() == "br") && !last_is_space(accum) {
                accum.push(' ');
            }
            for child in node.children() {
                walk(&child, accum);
            }
            let next = node.next_sibling();
            let next_inline =
                next.is_some_and(|n| n.value().is_text() || (n.value().is_element() && !formats_as_block(&n)));
            if is_block(node) && next_inline && !last_is_space(accum) {
                accum.push(' ');
            }
        }
        _ => {}
    }
}

/// `Element.ownText()`
pub fn own_text(element: &Element) -> String {
    let mut accum = String::new();
    for child in element.children() {
        match child.value() {
            Node::Text(text) => append_text(&mut accum, &child, text),
            Node::Element(e) if e.name() == "br" && !last_is_space(&accum) => accum.push(' '),
            _ => {}
        }
    }
    java_trim(&accum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsoup_text() {
        let page = Page::parse(
            "<html><head><title>  A \n - B @ mtgtop8.com </title></head><body>\
             <div class='x Event_Title'>#1 Deck -  <a href=x>Pla&amp;yer</a><br>next<p>para</p>after</div>\
             <div class=meta_arch>Modern <img src=a.png> <b>bold</b></div></body></html>",
        );
        assert_eq!(page.title(), "A - B @ mtgtop8.com");
        let event = page.first_div("event_title").unwrap();
        assert_eq!(text(&event), "#1 Deck - Pla&yer next para after");
        assert_eq!(own_text(&page.first_div("meta_arch").unwrap()), "Modern");
        assert!(page.first_div("missing").is_none());
    }
}
