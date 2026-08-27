use crate::matching::{Matches, ShortlistMatch};
use docx_rs::*;

const FONT: &str = "Source Sans Pro";
const SEMIBOLD_FONT: &str = "Source Sans Pro Semibold";

// US Letter, in twips (1/20 of a point)
const PAGE_WIDTH: u32 = 12240;
const PAGE_HEIGHT: u32 = 15840;

const MARGIN: i32 = 720; // 0.5"

const USABLE_WIDTH: usize = (PAGE_WIDTH - MARGIN as u32 * 2) as usize;

const LINE_SPACING: i32 = 240;

pub fn generate_docx(matches: &Matches) -> anyhow::Result<()> {
    let file = std::fs::File::create("./matches.docx")?;
    generate_docx_data(matches).pack(file)?;
    Ok(())
}

fn generate_docx_data(matches: &Matches) -> Docx {
    // Must set hi_ansi so that n dashes and spaces get the right styling.
    let run_font = RunFonts::new()
        .hi_ansi(FONT)
        .ascii(FONT)
        .cs(FONT)
        .east_asia(FONT);
    let semi_bold_run_font = RunFonts::new()
        .hi_ansi(SEMIBOLD_FONT)
        .ascii(SEMIBOLD_FONT)
        .cs(SEMIBOLD_FONT)
        .east_asia(SEMIBOLD_FONT);

    let cards = generate_cards(matches, &run_font, &semi_bold_run_font);
    let section = generate_section(cards);

    let styles = Styles::new()
        .default_fonts(run_font.clone())
        .default_size(18);

    Docx::new()
        .page_size(PAGE_WIDTH, PAGE_HEIGHT)
        .styles(styles)
        .add_section(section)
}

fn generate_cards(
    matches: &Matches,
    run_font: &RunFonts,
    semi_bold_run_font: &RunFonts,
) -> Vec<Table> {
    matches
        .cards
        .iter()
        .map(|card| {
            let cell = generate_table_cell();

            let name_paragraph = Paragraph::new()
                .line_spacing(LineSpacing::new().after(80).line(LINE_SPACING))
                .add_run(
                    Run::new()
                        .add_text(card.name.clone())
                        .bold()
                        .size(26)
                        .fonts(run_font.clone()),
                );
            let cell = cell.add_paragraph(name_paragraph);

            let your_matches_paragraph = Paragraph::new()
                .line_spacing(LineSpacing::new().after(100).line(LINE_SPACING))
                .set_borders(
                    ParagraphBorders::with_empty().set(
                        ParagraphBorder::new(ParagraphBorderPosition::Bottom)
                            .val(BorderType::Single)
                            .color("999999")
                            .size(4),
                    ),
                )
                .add_run(
                    Run::new()
                        .add_text("Your matches:")
                        .bold()
                        .size(18)
                        .color("444444")
                        .fonts(run_font.clone()),
                );
            let mut cell = cell.add_paragraph(your_matches_paragraph);

            for (i, item) in card.shortlist.iter().enumerate() {
                generate_match_info(i, item, &mut cell, run_font, semi_bold_run_font);
            }

            let row = TableRow::new(vec![cell]);
            Table::new(vec![row])
                .width(USABLE_WIDTH, WidthType::Dxa)
                .set_borders(generate_table_borders())
        })
        .collect()
}

fn generate_match_info(
    index: usize,
    item: &ShortlistMatch,
    cell: &mut TableCell,
    run_font: &RunFonts,
    semi_bold_run_font: &RunFonts,
) {
    let name_paragraph = Paragraph::new()
        .line_spacing(
            LineSpacing::new()
                .before(if index == 0 { 0 } else { 110 })
                .after(20)
                .line(LINE_SPACING),
        )
        .add_run(
            Run::new()
                .add_text(format!("{} – {}", item.name, item.age.0))
                .bold()
                .size(21)
                .fonts(run_font.clone()),
        )
        .add_run(
            Run::new()
                .add_text(format!("  ({})", item.email))
                .size(18)
                .color("555555")
                .fonts(run_font.clone()),
        );

    *cell = cell.clone().add_paragraph(name_paragraph);

    if item.freeresponse.responses.is_empty() {
        let no_responses_paragraph = Paragraph::new()
            .indent(Some(220), None, None, None)
            .line_spacing(LineSpacing::new().after(20).line(LINE_SPACING))
            .add_run(
                Run::new()
                    .add_text("(no profile answers on file)")
                    .italic()
                    .size(16)
                    .color("888888")
                    .fonts(run_font.clone()),
            );
        *cell = cell.clone().add_paragraph(no_responses_paragraph)
    } else {
        for (prompt, response) in &item.freeresponse.responses {
            let prompt_and_response_paragraph = Paragraph::new()
                .indent(Some(220), None, None, None)
                .line_spacing(LineSpacing::new().after(40).line(LINE_SPACING))
                .add_run(
                    Run::new()
                        .add_text(format!("{} ", prompt)) // Separator colon is baked into the prompt.
                        .size(18)
                        .fonts(semi_bold_run_font.clone()),
                )
                .add_run(
                    Run::new()
                        .add_text(response)
                        .size(18)
                        .fonts(run_font.clone()),
                );
            *cell = cell.clone().add_paragraph(prompt_and_response_paragraph);
        }
    }
}

fn generate_table_cell() -> TableCell {
    let mut cell = TableCell::new().width(USABLE_WIDTH, WidthType::Dxa);
    let property = cell
        .property
        .margin_top(140, WidthType::Dxa)
        .margin_bottom(140, WidthType::Dxa)
        .margin_left(220, WidthType::Dxa)
        .margin_right(220, WidthType::Dxa);
    cell.property = property;

    cell
}

fn generate_table_borders() -> TableBorders {
    let color = "888888";
    let size: usize = 6;
    let border_type = BorderType::Dashed;
    TableBorders::new()
        .set(
            TableBorder::new(TableBorderPosition::Top)
                .border_type(border_type)
                .size(size)
                .color(color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Bottom)
                .border_type(border_type)
                .size(size)
                .color(color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Left)
                .border_type(border_type)
                .size(size)
                .color(color),
        )
        .set(
            TableBorder::new(TableBorderPosition::Right)
                .border_type(border_type)
                .size(size)
                .color(color),
        )
}

fn generate_section(cards: Vec<Table>) -> Section {
    let page_size = PageSize::new().width(PAGE_WIDTH).height(PAGE_HEIGHT);
    let page_margin = PageMargin::new()
        .top(MARGIN)
        .bottom(MARGIN)
        .left(MARGIN)
        .right(MARGIN);
    let mut section = Section::new().page_size(page_size).page_margin(page_margin);

    let (chunks, remainder) = cards.as_chunks();
    for [card_a, card_b] in chunks {
        section = section.add_table(card_a.clone());

        // Empty line between 2 blocks on a page
        let empty_line_paragraph = Paragraph::new().add_run(Run::new().add_text(""));
        section = section.add_paragraph(empty_line_paragraph);

        section = section.add_table(card_b.clone());

        // Page break after two sections. Some longer cards will go onto the next page and will need to be manually
        // page-broken in the word doc.
        let page_break_paragraph = Paragraph::new().add_run(Run::new().add_break(BreakType::Page));
        section = section.add_paragraph(page_break_paragraph);
    }

    assert!(remainder.is_empty() || remainder.len() == 1);
    for card in remainder {
        section = section.add_table(card.clone());
    }

    section
}

#[cfg(test)]
mod tests {
    use crate::matching::{MatchCard, ShortlistMatch};
    use crate::parsing::{Age, FreeResponse};

    use super::*;

    #[test]
    fn docx_generated() {
        let matches = Matches {
            cards: vec![
                MatchCard {
                    name: "Candidate A".to_string(),
                    email: "first".to_string(),
                    shortlist: vec![ShortlistMatch {
                        name: "Candidate B".to_string(),
                        age: Age(26),
                        email: "second".to_string(),
                        freeresponse: FreeResponse { responses: vec![] },
                        score: 0.98976606,
                    }],
                },
                MatchCard {
                    name: "Candidate B".to_string(),
                    email: "second".to_string(),
                    shortlist: vec![ShortlistMatch {
                        name: "Candidate A".to_string(),
                        age: Age(34),
                        email: "first".to_string(),
                        freeresponse: FreeResponse { responses: vec![] },
                        score: 0.98976606,
                    }],
                },
            ],
            print_scores: true,
        };

        let data = generate_docx_data(&matches);
        assert_eq!(data.document.children.len(), 1);
        assert_eq!(data.build().document.len(), 6711);
    }
}
