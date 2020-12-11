#![feature(rustc_private)]

use std::env;
use std::collections::HashMap;
use rustc_hir::{intravisit::{self, Visitor}, self as hir};
use rustc_middle::ty::{TyKind, TyCtxt};
use rustc_middle::hir::map::Map;

extern crate rustc_data_structures;
extern crate rustc_session;
extern crate rustc_interface;
extern crate rustc_span;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_serialize;

type CallLocations = HashMap<String, HashMap<String, Vec<(u32, u32)>>>;

struct FindCalls<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    map: Map<'tcx>,
    file_name: String,
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
                let entries = self.calls.entry(key).or_insert(HashMap::new());
                entries.entry(self.file_name.clone()).or_insert(vec![]).push((ex.span.lo().0, ex.span.hi().0));
            }
        }
        intravisit::walk_expr(self, ex);
    }
}

struct Callbacks<'a> {
    file_name: String,
    calls: &'a mut CallLocations
}

impl<'a> rustc_driver::Callbacks for Callbacks<'a> {
    fn after_analysis<'tcx>(&mut self, _compiler: &rustc_interface::interface::Compiler, queries: &'tcx rustc_interface::Queries<'tcx>) -> rustc_driver::Compilation {
        queries.global_ctxt().unwrap().take().enter(|tcx| {
            let krate = tcx.hir().krate();
            let map = tcx.hir();
            let calls = &mut self.calls;
            let file_name = self.file_name.clone();
            let mut finder = FindCalls { calls, tcx, map, file_name };
            krate.visit_all_item_likes(&mut finder.as_deep_visitor());
        });

        rustc_driver::Compilation::Stop
    }
}

use std::process::Command;
use regex::Regex;
use std::fs;

fn main() {
    Command::new("cargo")
        .args(&["clean", "--target-dir", "target/debug/examples"])
        .output().unwrap();

    let stderr = Command::new("cargo")
        .args(&["check", "--examples", "-v"])
        .output().unwrap().stderr;
    let stderr = String::from_utf8(stderr).unwrap();

    let re = Regex::new(r"^\s*Running `(.*)`").unwrap();
    let mut calls = HashMap::new();
    let toolchain_path = env::var("HOME").unwrap() + "/.rustup/toolchains/nightly-x86_64-apple-darwin";

    let command_lines = stderr.split("\n").filter_map(|line| {
      re.captures(line).map(|cap| cap.get(1).unwrap().as_str().to_string())      
    });

    command_lines.for_each(|command| {
      let mut args: Vec<_> = command
          .split(" ")
          .collect();

      let cfg_flags: Vec<_> = args.iter().enumerate().filter(|(_, s)| **s == "--cfg").map(|(i, _)| i).collect();
      for i in cfg_flags.iter().rev() {
        args.remove(*i + 1);
        args.remove(*i);
      }
      args.extend(vec!["--sysroot", &toolchain_path]);
      
      let args: Vec<_> = args.into_iter().map(|arg| arg.to_string()).collect();

      let file_name = match command.split(" ").find(|s| s.starts_with("examples/") || s.starts_with("src/")) {
          Some(s) => s.to_string(),
          None => return
      };

      let mut callbacks = Callbacks { calls: &mut calls, file_name };

      rustc_driver::install_ice_hook();
      rustc_driver::catch_fatal_errors(|| {
        rustc_driver::RunCompiler::new(&args, &mut callbacks).run().unwrap();
      }).unwrap();
    });

    let json = rustc_serialize::json::encode(&calls).unwrap();
    fs::write(".call_locations.json", json).unwrap();
}