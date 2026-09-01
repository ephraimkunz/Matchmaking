use crate::Matches;
use anyhow::Result;
use petgraph::{
    dot::{Config, Dot},
    graphmap::DiGraphMap,
};
use std::path::{Path, PathBuf};
use std::{io::Write, process::Command};

/// # Errors
///
/// Returns an error if the graph file cannot be created.
pub fn generate_graph(matches: &Matches) -> Result<PathBuf> {
    let graph_path = Path::new("./graph.gv").to_path_buf();
    let mut graph_file = std::fs::File::create(&graph_path)?;

    let edges = matches.0.iter().flat_map(|m| {
        m.shortlist
            .iter()
            .map(|s| (m.name.as_str(), s.name.as_str()))
    });

    let graph = DiGraphMap::<&str, ()>::from_edges(edges);

    let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);
    write!(graph_file, "{dot:?}")?;

    let image_path = Path::new("graph.png").to_path_buf();
    Command::new("neato")
        .arg("-Tpng")
        .arg("-Gsplines=true")
        .arg("-Goverlap=prism")
        .arg("-Gsep=\"+12,12\"")
        .arg("-Nfontsize=11")
        .arg("-Nfontname=Helvetica")
        .arg("-Ecolor=gray40")
        .arg("-Earrowsize=0.6")
        .arg("-Epenwidth=0.8")
        .arg(&graph_path)
        .arg("-o")
        .arg(&image_path)
        .output()?;

    std::fs::remove_file(graph_path)?;

    Ok(image_path)
}
