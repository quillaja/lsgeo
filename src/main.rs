use clap::Parser;
use colored::*;
use gdal::vector::{LayerAccess, OGRFieldType, OGRwkbGeometryType};
use glob::{PatternError, glob};
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use openssl; // to make openssl/crypto c++ libs not freak out

fn main() {
    let args = Searcher::parse();

    for search_path in args.search_paths() {
        args.search_glob_path(search_path).unwrap_or_else(|err| {
            eprintln!("{}: {}", "Error".red(), err);
        });
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles=clap_themes::CARGO)]
struct Searcher {
    /// Path to begin search.
    // #[arg(short, long, default_value_t=String::from("."))]
    #[arg(default_value_t=String::from("."))]
    path: String,
    /// Recursively search sub directories.
    #[arg(short, long, default_value_t = false)]
    recurse: bool,
    /// Show field details (name, type, width, prec, nullable).
    #[arg(short, long, default_value_t = false)]
    fields: bool,
    /// File types to find.
    #[arg(short, long, num_args=0.., default_values=&[".gdb", ".shp", ".gpkg"])]
    exts: Vec<String>,
}

impl Searcher {
    fn search_paths(&self) -> impl Iterator<Item = PathBuf> {
        let recurse = if self.recurse { "**/" } else { "" };
        let mut paths: Vec<PathBuf> = Vec::new();
        for ext in &self.exts {
            let pattern = format!("{}/{}*{}", self.path, recurse, ext);
            let Ok(search_path) = std::path::absolute(Path::new(&pattern)) else {
                println!("{}: {}", "Error".red(), pattern);
                continue;
            };
            paths.push(search_path);
        }
        return paths.into_iter();
    }

    fn search_glob_path(&self, pattern: impl AsRef<std::path::Path>) -> Result<(), PatternError> {
        let pattern = pattern.as_ref().to_str().unwrap();
        let paths = glob(pattern)?;
        for path in paths {
            let dataset = match path {
                Ok(val) => val,
                Err(err) => {
                    eprintln!("{}: {}", "Failed".red(), err);
                    continue;
                }
            };
            self.print_dataset(&dataset);
        }
        Ok(())
    }

    fn print_dataset(&self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        println!("\n{}", path.display());
        let d = match gdal::Dataset::open_ex(path, gdal::DatasetOptions::default()) {
            Ok(val) => val,
            Err(err) => {
                eprintln!(
                    "{}: {} : {}",
                    "Failed to open".red(),
                    path.file_name().unwrap().display(),
                    err
                );
                return;
            }
        };

        for l in d.layers() {
            let lname = l.name();
            let def = l.defn();
            let gname = geom_name(def.geometry_type());
            let nfields = def.fields().count();
            println!("\t{:<32}{:<22}{:6} fields", lname, gname, nfields);

            if self.fields {
                for f in def.fields() {
                    println!(
                        "\t\t{:<30}{:<8}{:5}{:3}{:>6}",
                        f.name(),
                        field_type(f.field_type()),
                        f.width(),
                        f.precision(),
                        f.is_nullable()
                    )
                }
            }
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
