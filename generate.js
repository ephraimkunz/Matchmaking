const fs = require("fs");
const {
  Document, Packer, Paragraph, TextRun, PageBreak,
  BorderStyle, AlignmentType, Table, TableRow, TableCell, WidthType,
} = require("docx");

const FONT = "Source Sans Pro";
const SEMIBOLD_FONT = "Source Sans Pro Semibold";

const matches = JSON.parse(process.argv[2]);

const PAGE_WIDTH = 12240;   // US Letter, twips
const PAGE_HEIGHT = 15840;
const MARGIN = 720;         // 0.5"
const USABLE_WIDTH = PAGE_WIDTH - MARGIN * 2;

// Builds the paragraphs for one person's slip (their name + all their matches)
function buildSlip(person) {
  const children = [];

  children.push(
    new Paragraph({
      spacing: { after: 80, line: 240 },
      children: [
        new TextRun({ text: person.name, bold: true, size: 26, font: FONT }),
      ],
    })
  );
  children.push(
    new Paragraph({
      spacing: { after: 100, line: 240 },
      border: { bottom: { style: BorderStyle.SINGLE, size: 4, color: "999999" } },
      children: [new TextRun({ text: "Your matches:", bold: true, size: 18, color: "444444", font: FONT })],
    })
  );

  person.shortlist.forEach((match, i) => {
    children.push(
      new Paragraph({
        spacing: { before: i === 0 ? 0 : 110, after: 20, line: 240 },
        children: [
          new TextRun({ text: match.name + " – " + match.age, bold: true, size: 21, font: FONT }),
          new TextRun({ text: `  (${match.email})`, size: 18, color: "555555", font: FONT }),
        ],
      })
    );
    if (match.freeresponse.responses.length === 0) {
      children.push(
        new Paragraph({
          indent: { left: 220 },
          spacing: { after: 20, line: 240 },
          children: [new TextRun({ text: "(no profile answers on file)", italics: true, size: 16, color: "888888", font: FONT })],
        })
      );
    }
    match.freeresponse.responses.forEach((attr) => {
      children.push(
        new Paragraph({
          indent: { left: 220 },
          spacing: { after: 40, line: 240 },
          children: [
            new TextRun({ text: `${attr[0]}: `, size: 18, font: SEMIBOLD_FONT }),
            new TextRun({ text: attr[1], size: 18, font: FONT }),
          ],
        })
      );
    });
  });

  return children;
}

// Wrap a slip's content in a bordered box (the "cut along this box" outline)
function buildSlipBox(person) {
  return new Table({
    width: { size: USABLE_WIDTH, type: WidthType.DXA },
    columnWidths: [USABLE_WIDTH],
    borders: {
      top: { style: BorderStyle.DASHED, size: 6, color: "888888" },
      bottom: { style: BorderStyle.DASHED, size: 6, color: "888888" },
      left: { style: BorderStyle.DASHED, size: 6, color: "888888" },
      right: { style: BorderStyle.DASHED, size: 6, color: "888888" },
    },
    rows: [
      new TableRow({
        children: [
          new TableCell({
            width: { size: USABLE_WIDTH, type: WidthType.DXA },
            margins: { top: 140, bottom: 140, left: 220, right: 220 },
            children: buildSlip(person),
          }),
        ],
      }),
    ],
  });
}

// --- Estimate how many printed lines a person's slip will take, so we can
// pack pages without a slip splitting awkwardly across a page break. ---
const CHARS_PER_LINE = 100;
function wrapCount(text) {
  return Math.max(1, Math.ceil(text.length / CHARS_PER_LINE));
}
function estimateLines(person) {
  let lines = 2; // name header + "Your matches" rule
  for (const match of person.shortlist) {
    lines += wrapCount(`${match.name}  (${match.email})`);
    if (match.freeresponse.responses.length === 0) {
      lines += 1;
    } else {
      for (const attr of match.freeresponse.responses) {
        lines += wrapCount(`${attr[0]}: ${attr[1]}`);
      }
    }
  }
  return lines;
}

// A full page (two stacked slips + gap + box padding) comfortably holds
// about this many estimated lines. Tuned against rendered output.
const PAGE_CAPACITY = 90;

const body = [];
let i = 0;
while (i < matches.cards.length) {
  const first = matches.cards[i];
  const firstLines = estimateLines(first);
  const second = matches.cards[i + 1];
  const secondLines = second ? estimateLines(second) : 0;

  const fitsTwo = second && firstLines + secondLines <= PAGE_CAPACITY;

  body.push(buildSlipBox(first));
  if (fitsTwo) {
    body.push(new Paragraph({ text: "", spacing: { after: 0 } }));
    body.push(buildSlipBox(second));
    i += 2;
  } else {
    i += 1;
  }

  if (i < matches.cards.length) {
    body.push(new Paragraph({ children: [new PageBreak()] }));
  }
}

const doc = new Document({
  styles: {
    default: {
      document: {
        run: { font: FONT, size: 18 },
      },
    },
  },
  sections: [
    {
      properties: {
        page: {
          size: { width: PAGE_WIDTH, height: PAGE_HEIGHT },
          margin: { top: MARGIN, bottom: MARGIN, left: MARGIN, right: MARGIN },
        },
      },
      children: body,
    },
  ],
});

Packer.toBuffer(doc).then((buf) => {
  const out = process.argv[3] || "matches.docx";
  fs.writeFileSync(out, buf);
  console.log("Wrote", out);
});
