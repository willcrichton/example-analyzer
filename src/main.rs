#![feature(rustc_private)]

use std::collections::HashMap;
use rustc_data_structures::fx::{FxHashSet, FxHashMap};
use rustc_session::config::Options;
use std::path::PathBuf;
use rustc_hir::{intravisit::{self, Visitor}, self as hir};
use rustc_middle::ty::{TyKind, TyCtxt};
use rustc_middle::hir::map::Map;
use rustc_serialize::{Encoder, Encodable};

extern crate rustc_data_structures;
extern crate rustc_session;
extern crate rustc_interface;
extern crate rustc_span;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_serialize;

type CallLocations = HashMap<String, Vec<(u32, u32)>>;

// struct CallLocations {
//     file: String,
//     locations: Locations
// }

// impl<S: Encoder> Encodable<S> for CallLocations {
//     fn encode(&self, s: &mut S) -> Result<(), S::Error> {
//         s.emit_struct("CallLocations", 2, |s| {
//             s.emit_struct_field("file", 0, |s| self.file.encode(s))?;
//             s.emit_struct_field("locations", 1, |s| self.locations.encode(s))
//         })
//     }
// }

struct FindCalls<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    map: Map<'tcx>,
    calls: &'a mut CallLocations
}

impl<'a, 'tcx> Visitor<'tcx> for FindCalls<'a, 'tcx> where 'tcx: 'a {
    type Map = Map<'tcx>;

    fn nested_visit_map(&mut self) -> intravisit::NestedVisitorMap<Self::Map> { 
        intravisit::NestedVisitorMap::OnlyBodies(self.map)
    }

    fn visit_expr(&mut self, ex: &'tcx hir::Expr<'tcx>) {
        let types = self.tcx.typeck(ex.hir_id.owner);
        let ty = match ex.kind {
            hir::ExprKind::Call(f, _) => Some(types.node_type(f.hir_id)),
            hir::ExprKind::MethodCall(_, _, _, _) => Some({
                let types = self.tcx.typeck(ex.hir_id.owner);
                let def_id = types.type_dependent_def_id(ex.hir_id).unwrap();
                self.tcx.type_of(def_id)
            }),
            _ => None
        };

        if let Some(ty) = ty {
            if let TyKind::FnDef(def_id, _) = ty.kind() {
                let key = self.tcx.def_path_str(*def_id);
                self.calls.entry(key).or_insert(vec![]).push((ex.span.lo().0, ex.span.hi().0));
            }
        }
        intravisit::walk_expr(self, ex);
    }
}

struct Callbacks {
    calls: CallLocations
}

impl rustc_driver::Callbacks for Callbacks {
    fn after_analysis<'tcx>(&mut self, _compiler: &rustc_interface::interface::Compiler, queries: &'tcx rustc_interface::Queries<'tcx>) -> rustc_driver::Compilation {        
        queries.global_ctxt().unwrap().take().enter(|tcx| {
            let krate = tcx.hir().krate();
            let map = tcx.hir();
            let mut calls = &mut self.calls;
            let mut finder = FindCalls { calls, tcx, map };
            krate.visit_all_item_likes(&mut finder.as_deep_visitor());
        });

        rustc_driver::Compilation::Stop
    }
}

use std::process::Command;
use regex::Regex;
use std::fs;

fn main() {
    // let input = r#"fn main() { 
    //     let s = String::from("a");
    //     let mut v: Vec<i32> = Vec::new();
    //     v.push(1);
    //  }"#
    //     .to_string();    

    Command::new("cargo")
        .args(&["clean", "--target-dir", "target/debug/examples"])
        .output().unwrap();

    let stderr = Command::new("cargo")
        .args(&["check", "--examples", "-v"])
        .output().unwrap().stderr;
    let stderr = String::from_utf8(stderr).unwrap();

    let re = Regex::new(r"^\s*Running `(.*)`").unwrap();
    let commands: HashMap<_, _> = stderr
        .split("\n")
        .filter_map(|line| {
            re.captures(line).map(|cap| cap.get(1).unwrap().as_str().to_string())
        })
        .map(|command| {
            let args: Vec<_> = command
                .split(" ")
                .chain(vec!["--sysroot", "/Users/will/.rustup/toolchains/nightly-x86_64-apple-darwin"].into_iter())
                .map(|s| s.to_string())
                .collect();
            let file = &args[4];            
            let mut callbacks = Callbacks { calls: HashMap::new() };
            let runner = rustc_driver::RunCompiler::new(&args, &mut callbacks);
            runner.run().unwrap();

            (file.to_string(), callbacks.calls)
        })
        .collect();

    let json = rustc_serialize::json::encode(&commands).unwrap();
    fs::write(".call_locations.json", json).unwrap();
}