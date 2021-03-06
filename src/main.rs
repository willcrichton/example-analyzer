#![feature(rustc_private)]

use regex::Regex;
use rustc_hir::{
    self as hir,
    intravisit::{self, Visitor},
};
use rustc_middle::hir::map::Map;
use rustc_middle::ty::{TyCtxt, TyKind};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_serialize;
extern crate rustc_session;
extern crate rustc_span;

type CallLocations = HashMap<String, HashMap<String, Vec<(u32, u32)>>>;

struct FindCalls<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    map: Map<'tcx>,
    file_name: String,
    calls: &'a mut CallLocations,
}

impl<'a, 'tcx> Visitor<'tcx> for FindCalls<'a, 'tcx>
where
    'tcx: 'a,
{
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
            _ => None,
        };

        if let Some(ty) = ty {
            if let TyKind::FnDef(def_id, _) = ty.kind() {
                let key = format!(
                    "{}{}",
                    self.tcx.crate_name(def_id.krate),
                    self.tcx.def_path(*def_id).to_string_no_crate_verbose()
                );
                let entries = self.calls.entry(key).or_insert(HashMap::new());
                entries
                    .entry(self.file_name.clone())
                    .or_insert(vec![])
                    .push((ex.span.lo().0, ex.span.hi().0));
            }
        }
        intravisit::walk_expr(self, ex);
    }
}

struct Callbacks<'a> {
    file_name: String,
    calls: &'a mut CallLocations,
}

impl<'a> rustc_driver::Callbacks for Callbacks<'a> {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> rustc_driver::Compilation {
        queries.global_ctxt().unwrap().take().enter(|tcx| {
            let mut finder = FindCalls {
                calls: self.calls,
                tcx,
                map: tcx.hir(),
                file_name: self.file_name.clone(),
            };
            tcx.hir()
                .krate()
                .visit_all_item_likes(&mut finder.as_deep_visitor());
        });

        rustc_driver::Compilation::Stop
    }
}

// absolutely awful hack to fix the sysroot when running out-of-tree
// Taken from #78926, which took it from src/bin/miri.rs, which in turn took it from clippy ... rustc is a mess.
// FIXME(jyn514): implement https://github.com/rust-lang/rust/pull/78926#issuecomment-726653035 instead
// FIXME(jyn514): Why is this a compile-time constant? Won't that break when this is distributed?
// maybe use `rustc --print sysroot` instead?

/// Returns the "default sysroot" that will be used if no `--sysroot` flag is set.
/// Should be a compile-time constant.
fn compile_time_sysroot() -> String {
    /* NOTE: this doesn't matter for example-analyzer, which is always built out-of-tree,
     * but might matter if it's ever incorporated into rustdoc.
    if option_env!("RUSTC_STAGE").is_some() {
        // This is being built as part of rustc, and gets shipped with rustup.
        // We can rely on the sysroot computation in librustc_session.
        return None;
    }
    */
    // For builds outside rustc, we need to ensure that we got a sysroot
    // that gets used as a default.  The sysroot computation in librustc_session would
    // end up somewhere in the build dir (see `get_or_default_sysroot`).
    // Taken from PR <https://github.com/Manishearth/rust-clippy/pull/911>.
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => option_env!("RUST_SYSROOT").expect("to build example-analyzer without rustup, set RUST_SYSROOT to `rustc --print sysroot`").into(),
    }
}

fn main() {
    let sysroot = compile_time_sysroot();
    // Run Cargo to get the `rustc` commands used to check each example.
    // This gives us all the necessary flags (eg --extern) to get the example to compile.
    let cargo_output = {
        Command::new("cargo")
            .args(&["clean", "--target-dir", "target/debug/examples"])
            .output()
            .unwrap();

        let stderr = Command::new("cargo")
            .args(&["check", "--examples", "-v"])
            .output()
            .unwrap()
            .stderr;

        String::from_utf8(stderr).unwrap()
    };

    // Extract the `rustc` commands from the "Running `...`" stderr output.
    let command_lines = {
        let re = Regex::new(r"^\s*Running `(.*)`").unwrap();
        cargo_output
            .split("\n")
            .filter_map(|line| {
                re.captures(line)
                    .map(|cap| cap.get(1).unwrap().as_str().to_string())
            })
            .collect::<Vec<_>>()
    };

    // Find call locations for each example in the Cargo output
    let mut calls = HashMap::new();
    command_lines.iter().for_each(|command| {
        let mut args: Vec<_> = command
            .split(" ")
            .filter(|s| *s != "--error-format=json" && *s != "--json=diagnostic-rendered-ansi")
            .chain(vec!["--sysroot", &sysroot])
            .collect();

        // FIXME(willcrichton): when compiling warp, for some reason the --cfg flags
        // caused the rustc_driver to fail, but NOT rustc. Not sure what the discrepancy
        // in behavior is, but for now just remove the --cfg flags.
        let cfg_flags: Vec<_> = args
            .iter()
            .enumerate()
            .filter(|(_, s)| **s == "--cfg")
            .map(|(i, _)| i)
            .collect();
        for i in cfg_flags.iter().rev() {
            args.remove(*i + 1);
            args.remove(*i);
        }

        let args: Vec<_> = args.into_iter().map(|arg| arg.to_string()).collect();

        // Try to find file name from the arg list so we can save it in the call locations
        let file_name = match command
            .split(" ")
            .find(|s| s.starts_with("examples/") || s.starts_with("src/"))
        {
            Some(s) => s.to_string(),
            None => return,
        };

        let mut callbacks = Callbacks {
            calls: &mut calls,
            file_name,
        };

        // Run the compiler, finding any call locations
        rustc_driver::catch_fatal_errors(|| {
            rustc_driver::RunCompiler::new(&args, &mut callbacks)
                .run()
                .unwrap();
        })
        .unwrap();
    });

    // Save call locations to final output file
    let json = rustc_serialize::json::encode(&calls).unwrap();
    fs::write(".call_locations.json", json).unwrap();
    println!("Successfully wrote .call_locations.json");
}
