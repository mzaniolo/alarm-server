extern crate yaml_rust;
use yaml_rust::YamlLoader;
use std::fs;

pub fn load_config(path : &str) -> yaml_rust::Yaml
{
    let source= fs::read_to_string(path).unwrap();
    YamlLoader::load_from_str(&source).unwrap().remove(0)
}
