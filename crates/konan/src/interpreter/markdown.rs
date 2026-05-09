use crate::{
    interpreter::block_adornment::{
        HorizontalRule, ListItemBefore, TaskListBefore, horizontal_rule_print_command,
        list_item_before_print_command, set_heading_style, task_list_before_print_command,
    },
    printer::{AnyPrinter, RongtaPrinter},
};
use pulldown_cmark::{Options, Parser, Tag};

pub struct MarkdownInterpreter {
    printer: RongtaPrinter,
    list_indices: Vec<Option<u64>>,
}
impl MarkdownInterpreter {
    pub fn new(printer: RongtaPrinter) -> Self {
        Self {
            printer,
            list_indices: Vec::new(),
        }
    }

    fn handle_tag_start(&mut self, tag: &Tag) {
        match tag {
            Tag::Paragraph => {
                log::debug!("Tag start: Paragraph");
                self.printer.add_new_line();
                self.printer.reset_cached();
            }
            Tag::Heading {
                level,
                id: _,
                classes: _,
                attrs: _,
            } => {
                log::debug!("Tag start: Heading level {:?}", level);
                let level = match level {
                    pulldown_cmark::HeadingLevel::H1 => 1,
                    pulldown_cmark::HeadingLevel::H2 => 2,
                    pulldown_cmark::HeadingLevel::H3 => 3,
                    pulldown_cmark::HeadingLevel::H4 => 4,
                    pulldown_cmark::HeadingLevel::H5 => 5,
                    pulldown_cmark::HeadingLevel::H6 => 6,
                };
                self.printer.add_new_line();
                set_heading_style(level, &mut self.printer);
            }
            Tag::BlockQuote(_) | Tag::CodeBlock(_) => {
                log::debug!("Tag start: BlockQuote or CodeBlock");
                self.printer.add_new_line();
                self.printer.reset_cached();
                self.printer.set_cached_bold(true);
            }
            Tag::List(ordered_start) => {
                log::debug!("Tag start: List (ordered_start={:?})", ordered_start);
                self.list_indices.push(*ordered_start);
                self.printer.add_new_line();
            }
            Tag::Item => {
                let current = self.list_indices.last().copied().flatten();
                log::debug!("Tag start: Item (list_index={:?})", current);
                let before = match current {
                    Some(i) => {
                        let mut b = ListItemBefore::new_ordered(None);
                        b.next_index(i);
                        if let Some(Some(n)) = self.list_indices.last_mut() {
                            *n += 1;
                        }
                        b
                    }
                    None => ListItemBefore::new_unordered(),
                };
                list_item_before_print_command(before, &mut self.printer);
            }
            Tag::Strong => {
                log::debug!("Tag start: Strong");
                self.printer.set_cached_bold(true);
            }
            _ => {
                log::debug!("Tag start: unhandled {:?}", tag);
            }
        }
    }

    pub fn render_content(&mut self, markdown: &str) {
        for event in Parser::new_ext(markdown, Options::ENABLE_TASKLISTS) {
            match &event {
                pulldown_cmark::Event::Start(tag) => self.handle_tag_start(tag),
                pulldown_cmark::Event::End(tag) => {
                    use pulldown_cmark::TagEnd;
                    log::debug!("Event: End({:?})", tag);
                    match tag {
                        TagEnd::Paragraph
                        | TagEnd::Heading(_)
                        | TagEnd::BlockQuote(_)
                        | TagEnd::CodeBlock
                        | TagEnd::Item => {
                            self.printer.add_new_line();
                        }
                        TagEnd::List(_) => {
                            self.list_indices.pop();
                            self.printer.add_new_line();
                        }
                        _ => {}
                    }
                    continue;
                }
                pulldown_cmark::Event::Text(cow_str) => {
                    log::debug!("Event: Text(\"{}\")", cow_str);
                    self.printer.add_content(cow_str);
                    continue;
                }
                pulldown_cmark::Event::Code(code) => {
                    log::debug!("Event: Code(\"{}\")", code);
                    continue;
                }
                pulldown_cmark::Event::InlineMath(math) => {
                    log::debug!("Event: InlineMath(\"{}\")", math);
                    continue;
                }
                pulldown_cmark::Event::DisplayMath(math) => {
                    log::debug!("Event: DisplayMath(\"{}\")", math);
                    continue;
                }
                pulldown_cmark::Event::Html(html) => {
                    log::debug!("Event: Html(\"{}\")", html);
                    continue;
                }
                pulldown_cmark::Event::InlineHtml(html) => {
                    log::debug!("Event: InlineHtml(\"{}\")", html);
                    continue;
                }
                pulldown_cmark::Event::FootnoteReference(label) => {
                    log::debug!("Event: FootnoteReference(\"{}\")", label);
                    continue;
                }
                pulldown_cmark::Event::SoftBreak => {
                    log::debug!("Event: SoftBreak");
                    self.printer.add_new_line();
                    continue;
                }
                pulldown_cmark::Event::HardBreak => {
                    log::debug!("Event: HardBreak");
                    self.printer.add_new_line();
                    self.printer.add_new_line();
                    continue;
                }
                pulldown_cmark::Event::Rule => {
                    log::debug!("Event: Rule");
                    let r = HorizontalRule::new();
                    horizontal_rule_print_command(r, &mut self.printer);
                    continue;
                }
                pulldown_cmark::Event::TaskListMarker(checked) => {
                    log::debug!("Event: TaskListMarker(checked={})", checked);
                    let before = TaskListBefore::new(*checked);
                    task_list_before_print_command(before, &mut self.printer);
                    continue;
                }
            };
        }
    }

    pub fn print(&self, rows: Option<u32>, driver: AnyPrinter) -> escpos::errors::Result<()> {
        self.printer.print(rows, driver)?;
        log::info!("Printed markdown");
        Ok(())
    }
}
