const PREFIX: &str = "fn Java_dev_dov_mk_";

pub fn rust(sources: &[&str]) -> String {
    let mut s = String::from(
        "#[allow(clippy::vec_init_then_push)]\npub fn natives() -> Vec<(&'static str, &'static str, &'static str, *mut std::ffi::c_void)> {\n    let mut list = Vec::new();\n",
    );
    for source in sources {
        let mut cfg = None;
        for (i, line) in source.lines().enumerate() {
            if line.starts_with("#[cfg(") {
                cfg = Some(line);
                continue;
            }
            if !line.contains(PREFIX) {
                if !line.starts_with("#[cfg_attr(") {
                    cfg = None;
                }
                continue;
            }
            let rest: String = source.lines().skip(i).collect::<Vec<_>>().join(" ");
            let rest = &rest[rest.find(PREFIX).unwrap() + 3..];
            let name = &rest[..rest.find(['<', '(']).unwrap()];
            let (class, method) = name["Java_dev_dov_mk_".len()..].split_once('_').unwrap();
            let open = rest.find('(').unwrap();
            let close = rest.find(')').unwrap();
            let params: String = rest[open + 1..close]
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .skip(2)
                .map(|p| java(p.split_once(':').unwrap().1.trim()))
                .collect();
            let tail = &rest[close + 1..rest.find('{').unwrap()];
            let ret = tail.split_once("->").map_or("V", |(_, t)| java(t.trim()));
            if let Some(c) = cfg {
                s += &format!("    {c}\n");
            }
            s += &format!(
                "    list.push((\"dev/dov/mk/{class}\", \"{method}\", \"({params}){ret}\", {name} as *mut std::ffi::c_void));\n"
            );
            cfg = None;
        }
    }
    s + "    list\n}\n"
}

fn java(kind: &str) -> &'static str {
    match kind {
        "jlong" => "J",
        "jint" => "I",
        "jfloat" => "F",
        "jboolean" => "Z",
        k if k.contains("JString") => "Ljava/lang/String;",
        k => panic!("no JNI signature for {k}"),
    }
}
