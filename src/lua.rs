use mlua::{Lua, ObjectLike, Value};
use std::path::PathBuf;
use std::{env, fs};
use tabled::{
    builder::Builder,
    settings::{Alignment, Style},
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

fn print_two_dimensional_table(tab: &LuaTable, tab_col: &LuaTable) -> mlua::Result<String> {
    let mut builder = Builder::new();
    let mut columns = Vec::with_capacity(16);

    columns.push("(i)".to_string());
    for (_, cols) in tab_col.pairs::<mlua::Value, LuaTable>().flatten() {
        columns.push(cols.get("name")?);
    }
    builder.push_record(columns.clone());

    for (key, tab2d) in tab.pairs::<mlua::Integer, LuaTable>().flatten() {
        let mut record: Vec<String> = Vec::with_capacity(columns.len());
        record.push((key - 1).to_string());

        for (_index, value) in tab2d.pairs::<mlua::Integer, LuaTable>().flatten() {
            record.push(value.get("data")?);
        }
        builder.push_record(record);
    }

    Ok(builder
        .build()
        .with((Alignment::right(), Style::rounded()))
        .to_string())
}

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
        println!("path: {:?}", entry.path());
        let file_content = fs::read_to_string(entry.path())?;
        lua.load(&file_content).exec()?;
    }

    let structure_obj = globals.get::<LuaTable>("Structure")?;
    let match_table: LuaTable = structure_obj.get("match_table")?;

    let alias_lstr: mlua::String = globals.call_function("LoopMatchAlias", (origin_name, match_table))?;
    let alias = alias_lstr.to_string_lossy();
    if alias.is_empty() {
        return Err(mlua::Error::runtime("Empty alias"));
    }

    let lua_str = lua.create_string(bytes)?;
    let create_func = format!("new_{}", alias);
    let col_name = format!("{}_col", alias);

    let instance: LuaTable = structure_obj.call_method(&create_func, lua_str)?;

    let inner: LuaTable = instance.get(alias)?;
    let inner_col: LuaTable = instance.get(col_name)?;

    print_two_dimensional_table(&inner, &inner_col)
}
