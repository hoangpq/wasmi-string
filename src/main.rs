extern crate wasmi;

use wasmi::{
    Module, ModuleInstance,
    ImportsBuilder, NopExternals,
    RuntimeValue, MemoryDescriptor,
    GlobalInstance, GlobalRef,
    MemoryRef, TableRef, MemoryInstance,
    TableInstance, ModuleRef,
    ModuleImportResolver, Error,
    GlobalDescriptor, Signature,
    FuncRef, TableDescriptor,
};

use wasmi::memory_units::Pages;
use std::{env, str};
use std::fs::File;

fn load_from_file(filename: &str) -> Module {
    use std::io::prelude::*;
    let mut file = File::open(filename).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    Module::from_buffer(buf).unwrap()
}

struct Env {
    table_base: GlobalRef,
    memory_base: GlobalRef,
    memory: MemoryRef,
    table: TableRef,
}

impl Env {
    fn new() -> Env {
        Env {
            table_base: GlobalInstance::alloc(RuntimeValue::I32(0), false),
            memory_base: GlobalInstance::alloc(RuntimeValue::I32(0), false),
            memory: MemoryInstance::alloc(Pages(1), None).unwrap(),
            table: TableInstance::alloc(64, None).unwrap(),
        }
    }
}

impl ModuleImportResolver for Env {
    fn resolve_func(&self, _field_name: &str, _func_type: &Signature) -> Result<FuncRef, Error> {
        Err(Error::Instantiation(
            "env module doesn't provide any functions".into(),
        ))
    }

    fn resolve_global(
        &self,
        field_name: &str,
        _global_type: &GlobalDescriptor,
    ) -> Result<GlobalRef, Error> {
        match field_name {
            "tableBase" => Ok(self.table_base.clone()),
            "memoryBase" => Ok(self.memory_base.clone()),
            _ => Err(Error::Instantiation(format!(
                "env module doesn't provide global '{}'",
                field_name
            ))),
        }
    }

    fn resolve_memory(
        &self,
        field_name: &str,
        _memory_type: &MemoryDescriptor,
    ) -> Result<MemoryRef, Error> {
        match field_name {
            "memory" => Ok(self.memory.clone()),
            _ => Err(Error::Instantiation(format!(
                "env module doesn't provide memory '{}'",
                field_name
            ))),
        }
    }

    fn resolve_table(
        &self,
        field_name: &str,
        _table_type: &TableDescriptor,
    ) -> Result<TableRef, Error> {
        match field_name {
            "table" => Ok(self.table.clone()),
            _ => Err(Error::Instantiation(format!(
                "env module doesn't provide table '{}'",
                field_name
            ))),
        }
    }
}

fn new_string(instance: &ModuleRef, memory: &MemoryRef, s: String) -> u32 {
    let result = instance
        .invoke_export("alloc", &[RuntimeValue::I32((s.len() + 1) as i32)], &mut NopExternals);

    let _len = s.len() as i32;
    match result.unwrap().unwrap() {
        RuntimeValue::I32(val) => {
            let bytes = s.as_bytes();
            let len = bytes.len();
            for i in 0..len {
                memory.set_value((val + i as i32) as u32, bytes[i]).unwrap();
            }
            memory.set_value((val + len as i32) as u32, 0u8).unwrap();
            val as u32
        }
        _ => 0 as u32
    }
}

fn get_string(instance: &ModuleRef, memory: &MemoryRef, mut ptr: u32) -> String {
    let mut bytes: Vec<u8> = vec![];
    loop {
        let mut buf = [0u8; 1];
        match memory.get_into(ptr, &mut buf) {
            Ok(_) => {
                if buf[0] != 0 {
                    bytes.push(buf[0]);
                    ptr = ptr + 1
                } else {
                    break;
                }
            }
            Err(_) => {}
        }
    }
    let _result = instance
        .invoke_export("dealloc",
                       &[RuntimeValue::I32(ptr as i32), RuntimeValue::I32(bytes.len() as i32)],
                       &mut NopExternals);

    String::from_utf8(bytes).unwrap()
}

fn main() {
    let path = env::current_dir().unwrap();
    let module = load_from_file(format!("{}/wasm/sha1.wasm", path.display()).as_str());
    let env = Env::new();

    let instance = ModuleInstance::new(
        &module, &ImportsBuilder::new().with_resolver("env", &env))
        .expect("Failed to instantiate module")
        .run_start(&mut NopExternals)
        .expect("Failed to run start function in module");

    let memory = instance.export_by_name("memory")
        .expect("`memory` export not found")
        .as_memory()
        .expect("export name `memory` is not of memory type")
        .to_owned();

    let _s2 = String::from_utf8(vec![
        76, 195, 182, 119, 101, 32, 232, 128, 129, 232,
        153, 142, 32, 76, 195, 169, 111, 112, 97, 114, 100
    ]).unwrap();

    let p = new_string(&instance, &memory, _s2.clone());
    let result = instance
        .invoke_export("digest", &[RuntimeValue::I32(p as i32)], &mut NopExternals);

    match result {
        Ok(e) => {
            match e.unwrap() {
                RuntimeValue::I32(val) => {
                    let s = get_string(&instance, &memory, val as u32);
                    assert_eq!("b12ca521f06bb949e47a0cc05656c9075bca63ed", s);
                    println!("Sha1 of `{}` is `{}`", _s2.clone(), s);
                }
                _ => println!("Not implemented yet")
            }
        }
        Err(e) => match e {
            _ => println!("Not implemented yet")
        }
    }
}
