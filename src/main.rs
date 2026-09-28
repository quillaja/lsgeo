use clap::{Parser, ValueEnum};
use colored::*;
use gdal::vector::{LayerAccess, OGRFieldType, OGRwkbGeometryType};
use glob::{Paths, glob};
use std::{
    error::Error,
    iter,
    path::{Path, PathBuf},
    vec,
};

#[allow(unused_imports)]
use openssl; // to make openssl/crypto c++ libs not freak out

fn main() {
    let args = Searcher::parse();
    colored::control::set_override(!args.mono);

    args.paths()
        .map(Dataset::new)
        .for_each(|ds| args.print_dataset(&ds));
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Format {
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles=clap_themes::CARGO)]
struct Searcher {
    /// Path to begin search.
    #[arg(default_value_t=String::from("."))]
    path: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t=Format::Text)]
    format: Format,
    /// Recursively search sub directories.
    #[arg(short, long, default_value_t = false)]
    recurse: bool,
    /// Show field details (name, type, width, prec, nullable).
    #[arg(short, long, default_value_t = false)]
    fields: bool,
    /// Do not color text output.
    #[arg(short, long, default_value_t = false)]
    mono: bool,
    /// Show errors.
    #[arg(long, default_value_t = false)]
    errors: bool,
    /// File types to find.
    #[arg(short, long, num_args=0.., default_values=&[".gdb", ".shp", ".gpkg"])]
    exts: Vec<String>,
}

impl Searcher {
    fn paths(&self) -> impl Iterator<Item = PathBuf> {
        let mut all_path_iters: Vec<Paths> = Vec::new();

        for pattern in self.search_patterns() {
            let result = glob(pattern.to_str().unwrap());
            let Ok(paths) = result else {
                self.print_error(result.unwrap_err());
                continue;
            };
            all_path_iters.push(paths);
        }

        let mut lazy_paths = all_path_iters.into_iter().flatten();

        // this is clunky...
        iter::from_fn(move || {
            loop {
                match lazy_paths.next() {
                    None => break None,
                    Some(presults) => match presults {
                        Ok(p) => break Some(p),
                        Err(e) => {
                            self.print_error(e);
                            continue;
                        }
                    },
                }
            }
        })
    }

    fn search_patterns(&self) -> impl Iterator<Item = PathBuf> {
        let recurse = if self.recurse { "**/" } else { "" };
        let mut search_patterns: Vec<PathBuf> = Vec::new();
        for ext in &self.exts {
            let pattern = format!("{}/{}*{}", self.path, recurse, ext);
            let Ok(abs_pattern) = std::path::absolute(Path::new(&pattern)) else {
                println!("{}: {}", "Error".red(), pattern);
                continue;
            };
            search_patterns.push(abs_pattern);
        }
        return search_patterns.into_iter();
    }

    fn print_error(&self, err: impl Error) {
        if self.errors {
            eprintln!("{}: {}", "Error".red(), err);
        }
    }

    fn print_dataset(&self, ds: &Dataset) {
        println!("\n{}", ds.path.green());
        if let Some(err) = ds.err.clone() {
            eprintln!(
                "{}: {}\n\t{}",
                "Failed to open".red(),
                Path::new(&ds.path).file_name().unwrap().display(),
                err
            );
            return;
        }

        for l in &ds.layers {
            println!(
                "\t{:<32}{:<22}{:6} fields",
                l.name,
                geom_name(l.geomtype),
                l.fields.len()
            );

            if self.fields {
                for f in &l.fields {
                    println!(
                        "\t\t{:<30}{:<8}{:5}{:3}{:>6}",
                        f.name,
                        field_type(f.datatype),
                        f.width,
                        f.precision,
                        f.nullable
                    )
                }
            }
        }
    }
}

#[derive(Debug)]
struct Dataset {
    path: String,
    err: Option<String>,
    layers: Vec<Layer>,
}

impl Dataset {
    fn new(path: impl AsRef<Path>) -> Self {
        let path = String::from(path.as_ref().to_string_lossy());
        match gdal::Dataset::open_ex(&path, gdal::DatasetOptions::default()) {
            Ok(ds) => Dataset {
                path: path,
                err: None,
                layers: ds.layers().map(Layer::new).collect(),
            },
            Err(err) => Dataset {
                path: path,
                err: Some(err.to_string()),
                layers: vec![],
            },
        }
    }
}

#[derive(Debug)]
struct Layer {
    name: String,
    geomtype: u32,
    fields: Vec<Field>,
}

impl Layer {
    fn new(layer: gdal::vector::Layer) -> Self {
        let def = layer.defn();
        Layer {
            name: layer.name(),
            geomtype: def.geometry_type(),
            fields: def.fields().map(Field::new).collect(),
        }
    }
}

#[derive(Debug)]
struct Field {
    name: String,
    datatype: u32,
    width: i32,
    precision: i32,
    nullable: bool,
}

impl Field {
    fn new(field: gdal::vector::Field) -> Self {
        Field {
            name: field.name(),
            datatype: field.field_type(),
            width: field.width(),
            precision: field.precision(),
            nullable: field.is_nullable(),
        }
    }
}

fn geom_name(geom_type: u32) -> &'static str {
    match geom_type {
        OGRwkbGeometryType::wkbPoint => "Point",
        OGRwkbGeometryType::wkbPointM => "PointM",
        OGRwkbGeometryType::wkbPointZM => "PointZM",
        OGRwkbGeometryType::wkbPoint25D => "Point25D",

        OGRwkbGeometryType::wkbLineString => "LineString",
        OGRwkbGeometryType::wkbLineStringM => "LineStringZ",
        OGRwkbGeometryType::wkbLineStringZM => "LineStringZM",
        OGRwkbGeometryType::wkbLineString25D => "LineString25D",

        OGRwkbGeometryType::wkbPolygon => "Polygon",
        OGRwkbGeometryType::wkbPolygonM => "PolygonM",
        OGRwkbGeometryType::wkbPolygonZM => "PolygonZM",
        OGRwkbGeometryType::wkbPolygon25D => "Polygon25D",

        OGRwkbGeometryType::wkbMultiPoint => "MultiPoint",
        OGRwkbGeometryType::wkbMultiPointM => "MultiPointM",
        OGRwkbGeometryType::wkbMultiPointZM => "MultiPointZM",
        OGRwkbGeometryType::wkbMultiPoint25D => "MultiPoint25D",

        OGRwkbGeometryType::wkbMultiLineString => "MultiLineString",
        OGRwkbGeometryType::wkbMultiLineStringM => "MultiLineStringM",
        OGRwkbGeometryType::wkbMultiLineStringZM => "MultiLineStringZM",
        OGRwkbGeometryType::wkbMultiLineString25D => "MultiLineString25D",

        OGRwkbGeometryType::wkbMultiPolygon => "MultiPolygon",
        OGRwkbGeometryType::wkbMultiPolygonM => "MultiPolygonM",
        OGRwkbGeometryType::wkbMultiPolygonZM => "MultiPolygonZM",
        OGRwkbGeometryType::wkbMultiPolygon25D => "MultiPolygon25D",

        OGRwkbGeometryType::wkbGeometryCollection => "GeometryCollection",
        OGRwkbGeometryType::wkbGeometryCollectionM => "GeometryCollectionM",
        OGRwkbGeometryType::wkbGeometryCollectionZM => "GeometryCollectionZM",
        OGRwkbGeometryType::wkbGeometryCollection25D => "GeometryCollection25D",

        _ => "Unknown",
    }
}

fn field_type(field_type: u32) -> &'static str {
    match field_type {
        OGRFieldType::OFTInteger => "int",
        OGRFieldType::OFTInteger64 => "int64",

        OGRFieldType::OFTReal => "real",

        OGRFieldType::OFTString => "text",

        OGRFieldType::OFTDate => "date",
        OGRFieldType::OFTDateTime => "datetime",
        OGRFieldType::OFTTime => "time",

        _ => "Other",
    }
}
