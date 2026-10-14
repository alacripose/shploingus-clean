//! The stdio server: an MCP client writes requests on stdin and reads replies on stdout.
//! A failure to serve is written to stderr, because stdout carries the protocol and nothing else.
#![warn(missing_docs)]

use std::io::{stdin, stdout, BufReader};

fn main() {
    let root = voxel_mcp::workspace_root();
    let input = BufReader::new(stdin().lock());
    let mut output = stdout().lock();
    if let Err(why) = voxel_mcp::serve(&root, input, &mut output) {
        eprintln!("voxel-mcp: {why}");
        std::process::exit(1);
    }
}
