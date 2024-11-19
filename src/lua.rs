use mlua::{Lua, ObjectLike, Value};
use std::path::PathBuf;
use std::{env, fs};
use tabled::{
    builder::Builder,
    // settings::Style
};
use walkdir::WalkDir;

const RCHEAT_CORE_SRC: &str = include_str!("../lua/core.lua");

type LuaTable = mlua::Table;

#[allow(dead_code)]
fn print_table(tab: &LuaTable, indent: usize) -> mlua::Result<String> {
    let prefix = vec!["  "; indent].concat();
    let mut output: Vec<_> = Vec::with_capacity(16);

    for pair in tab.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let key_str = match key {
            Value::Integer(i) => i.to_string(),
            Value::String(s) => s.to_string_lossy().to_string(),
            _ => String::from("nil"),
        };

        match value {
            Value::Table(child) => {
                output.push(format!("{}{}:\n", prefix, key_str));
                output.push(print_table(&child, indent + 1)?);
            }
            Value::Integer(integer) => output.push(format!("{}{}: {}\n", prefix, key_str, integer)),
            Value::String(s) => output.push(format!("{}{}: {}\n", prefix, key_str, s.to_string_lossy())),
            _ => (),
        }
    }
    Ok(output.concat())
}

#[allow(dead_code)]
fn print_two_dimensional_table(tab: &LuaTable, builder: &mut Builder) -> mlua::Result<String> {
    for (key, tab2d) in tab.pairs::<mlua::Integer, LuaTable>().flatten() {
        let mut record: Vec<String> = Vec::with_capacity(16);
        record.push(key.to_string());
        for (_index, value) in tab2d.pairs::<mlua::Integer, LuaTable>().flatten() {
            record.push(value.get("data")?);
        }
        // builder.insert_column(0, column);
    }
    Ok(builder.clone().build().to_string())
}

#[allow(dead_code)]
pub fn dump_with_lua(lua_src_path: &PathBuf, bytes: &[u8], origin_name: &str) -> mlua::Result<String> {
    env::set_current_dir(lua_src_path)?;

    // This loads the default Lua std library *without* the debug library.
    let lua = Lua::new();
    let globals = lua.globals();

    let walker = WalkDir::new(lua_src_path)
        .max_depth(1)
        .follow_links(true)
        .into_iter();

    lua.load(RCHEAT_CORE_SRC).exec()?;

    for entry in walker.flatten() {
        if entry.path().is_dir() {
            continue;
        }
        println!("path: {:?}, fname: {:?}", entry.path(), entry.file_name());
        let file_content = fs::read_to_string(entry.path())?;
        lua.load(&file_content).exec()?;
    }

    let method_res: mlua::String = globals
        .get::<LuaTable>("Structure")?
        .call_method("get_name", origin_name)?;

    // println!("{:?}", method_res);
    let byname = method_res.to_string_lossy();
    let create_func = format!("new_{}", byname);
    let lua_str = lua.create_string(bytes)?;

    let _res: LuaTable = globals
        .get::<LuaTable>("Structure")?
        .call_function(create_func.as_ref(), lua_str)?;

    let inner: LuaTable = _res.get(origin_name)?;

    let mut builder = Builder::new();
    print_two_dimensional_table(&inner, &mut builder)
}
