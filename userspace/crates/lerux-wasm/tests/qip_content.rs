//! Host probe for one QIP Content module.
//!
//! The module bytes are Apache-2.0. The notice is `fixtures/NOTICE`.
//! `run` still looks up `start`. This test calls `render` beside that entry.

use lerux_wasm::{run, Error, Imports};
use wasmi::{Engine, Instance, Linker, Memory, Module, Store};

const WASM: &[u8] = include_bytes!("fixtures/rgb-to-hex.wasm");
const INPUT: &[u8] = b"rgb(101, 79, 240)";
const OUTPUT: &[u8] = b"#654ff0";

struct Silent;

impl Imports for Silent {
    fn log(&mut self, _bytes: &[u8]) {}

    fn read(&mut self, _path: &[u8], _dst: &mut [u8]) -> i32 {
        -1
    }

    fn socket(&mut self) -> i32 {
        -1
    }
}

#[test]
fn run_still_requires_start() {
    assert!(WASM.len() <= lerux_wasm::MAX_WASM_LEN);
    assert_eq!(WASM.len(), 972);
    assert_eq!(run(WASM, &mut Silent), Err(Error::Export));
}

#[test]
fn render_packs_the_hex_bytes() {
    let engine = Engine::default();
    let module = Module::new(&engine, WASM).expect("module");
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_and_start(&mut store, &module)
        .expect("instance");
    assert_eq!(getter(&mut store, instance, "input_ptr"), 0x1_0000);
    assert_eq!(getter(&mut store, instance, "input_utf8_cap"), 0x1_0000);
    assert_eq!(getter(&mut store, instance, "output_utf8_cap"), 7);
    let memory = instance.get_memory(&store, "memory").expect("memory");
    assert_eq!(memory.data(&store).len(), 196_608);
    let packed = write_and_render(&mut store, instance, memory);
    assert_hex(&store, memory, packed);
}

fn getter(store: &mut Store<()>, instance: Instance, name: &str) -> i32 {
    instance
        .get_typed_func::<(), i32>(&*store, name)
        .unwrap_or_else(|_| panic!("export {name}"))
        .call(store, ())
        .unwrap_or_else(|_| panic!("call {name}"))
}

fn write_and_render(store: &mut Store<()>, instance: Instance, memory: Memory) -> i64 {
    let offset = usize::try_from(getter(store, instance, "input_ptr")).expect("input pointer");
    memory.write(&mut *store, offset, INPUT).expect("write");
    let render = instance
        .get_typed_func::<i32, i64>(&*store, "render")
        .expect("render");
    let len = i32::try_from(INPUT.len()).expect("input length");
    render.call(&mut *store, len).expect("render call")
}

fn assert_hex(store: &Store<()>, memory: Memory, packed: i64) {
    let bits = u64::from_le_bytes(packed.to_le_bytes());
    assert_eq!(bits >> 63, 0);
    let ptr = usize::try_from((bits >> 32) & 0x7FFF_FFFF).expect("output pointer");
    let len = usize::try_from(bits & 0xFFFF_FFFF).expect("output length");
    assert_eq!((ptr, len), (0x2_0000, OUTPUT.len()));
    let data = memory.data(store);
    assert_eq!(&data[ptr..ptr + len], OUTPUT);
}
