use blend_parse::Blend;
use std::env;
use std::path;

fn main() {
    let base_path = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let blend_path = base_path.join("examples/simple.blend");

    let blend = Blend::from_path(blend_path).unwrap();

    let unique_codes: std::collections::HashSet<_> = blend
        .blocks
        .iter()
        .map(|block| {
            if block.header.code[3] == 0 {
                String::from_utf8_lossy(&block.header.code[..2])
            } else {
                String::from_utf8_lossy(&block.header.code[..])
            }
        })
        .collect();

    println!("{:#?}", unique_codes);
    /*
    todo: add this to docs
    {
        "TEST",
        "SN",
        "GLOB",
        "LA",
        "WM",
        "IM",
        "ME",
        "DATA",
        "WS",
        "CA",
        "REND",
        "SC",
        "MA",
        "OB",
        "WO",
        "LS",
        "GR",
        "DNA1",
        "BR"
    }
    */
}
