use crate::cli::Cli;
use comfy_table::presets::ASCII_MARKDOWN;
use comfy_table::Table;
use csv::Writer;
use serde_json;
use sitemaps::{Entries, SitemapsFile};
use std::{error::Error, io::Write};
use tabwriter::TabWriter;

const HEADERS: [&str; 4] = ["loc", "lastmod", "changefreq", "priority"];

// TODO: move this serialization logic into the library and out of the cli
// challenge will be removing the dependency on the Cli struct
pub(crate) fn build_output(sitemap: SitemapsFile, cli: &Cli) -> Result<String, serde_json::Error> {
    if cli.json {
        return serde_json::to_string_pretty(&sitemap);
    }

    let (headers, columns) = build_headers_and_columns(&sitemap, cli);
    let rows = transpose_columns(columns);

    if cli.csv {
        return Ok(write_csv(headers, rows).unwrap());
    }

    if cli.markdown {
        return Ok(markdown(headers, rows, cli.header));
    }

    if cli.pretty {
        Ok(pretty(headers, rows, cli.header))
    } else {
        Ok(plain(headers, rows, cli.header))
    }
}

fn markdown(headers: Vec<&str>, rows: Vec<Vec<String>>, show_header: bool) -> String {
    let mut table = Table::new();
    table.load_preset(ASCII_MARKDOWN);

    if show_header {
        table.set_header(headers);
    }

    for row in rows {
        table.add_row(row);
    }

    format!("{table}")
}

fn pretty(headers: Vec<&str>, rows: Vec<Vec<String>>, show_header: bool) -> String {
    let mut table = Table::new();

    if show_header {
        table.set_header(headers);
    }

    for row in rows {
        table.add_row(row);
    }

    format!("{table}")
}

fn plain(headers: Vec<&str>, rows: Vec<Vec<String>>, show_header: bool) -> String {
    let mut tw = TabWriter::new(vec![]);

    let lines = rows
        .iter()
        .map(|row| row.join("\t"))
        .collect::<Vec<String>>();
    let buf = lines.join("\n");

    let output = if show_header {
        format!("{}\n{}", headers.join("\t"), buf)
    } else {
        buf
    };

    // I am skeptical of these unwraps, but I think the logic used in
    // `build_rows` might prevent panicking
    tw.write_all(output.as_bytes()).unwrap();
    tw.flush().unwrap();

    String::from_utf8(tw.into_inner().unwrap()).unwrap()
}

fn transpose_columns(columns: Vec<Vec<String>>) -> Vec<Vec<String>> {
    // TODO: get rid of clone
    (0..columns[0].len())
        .map(|i| {
            columns
                .iter()
                .map(|c| c[i].clone())
                .collect::<Vec<String>>()
        })
        .collect()
}

fn build_headers_and_columns(
    sitemap: &SitemapsFile,
    cli: &Cli,
) -> (Vec<&'static str>, Vec<Vec<String>>) {
    let mut headers = vec![];
    let mut columns = vec![];

    if cli.loc {
        headers.push(HEADERS[0]);
        let locs = sitemap.locs();
        columns.push(locs);
    }
    if cli.lastmod {
        headers.push(HEADERS[1]);
        let lastmods = sitemap.lastmods();
        columns.push(lastmods);
    }
    if let SitemapsFile::Sitemap(sitemap) = sitemap {
        if cli.changefreq && sitemap.entries.iter().any(|url| url.change_freq.is_some()) {
            headers.push(HEADERS[2]);
            let changefreqs = sitemap
                .entries
                .iter()
                .map(|url| {
                    if let Some(changefreq) = url.change_freq {
                        changefreq.to_string()
                    } else {
                        String::new()
                    }
                })
                .collect::<Vec<String>>();

            columns.push(changefreqs);
        }
        if cli.priority && sitemap.entries.iter().any(|url| url.priority.is_some()) {
            headers.push(HEADERS[3]);
            let priorities = sitemap
                .entries
                .iter()
                .map(|url| {
                    if let Some(priority) = url.priority {
                        priority.to_string()
                    } else {
                        String::new()
                    }
                })
                .collect::<Vec<String>>();

            columns.push(priorities);
        }
    }

    (headers, columns)
}

// TODO: tighten up this error handling
fn write_csv(headers: Vec<&str>, rows: Vec<Vec<String>>) -> Result<String, Box<dyn Error>> {
    let mut out = Writer::from_writer(vec![]);

    out.serialize(headers)?;
    for row in rows {
        out.serialize(row)?;
    }

    out.flush()?;

    Ok(String::from_utf8(out.into_inner()?)?)
}
