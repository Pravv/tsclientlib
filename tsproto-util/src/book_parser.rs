use ::*;

#[derive(Template)]
#[TemplatePath = "src/BookDeclarations.tt"]
#[derive(Deserialize, Debug)]
pub struct BookDeclarations {
    #[serde(rename = "struct")]
    pub(crate) structs: Vec<Struct>,
}

impl Declaration for BookDeclarations {
    type Dep = ();

    fn get_filename() -> &'static str { "BookDeclarations.toml" }

    fn parse(s: &str, (): Self::Dep) -> Self {
        ::toml::from_str(s).unwrap()
    }
}

impl BookDeclarations {
    pub fn get_struct(&self, name: &str) -> &Struct {
        if let Some(s) = self.structs.iter().find(|s| s.name == name) {
            s
        } else {
            panic!("Cannot find bookkeeping struct {}", name);
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Accessors {
    pub get: bool,
    pub set: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Id {
    #[serde(rename = "struct")]
    pub struct_name: String,
    pub prop: String,
}

impl Id {
    pub fn find_property<'a>(&self, structs: &'a [Struct]) -> &'a Property {
        // Find struct
        for s in structs {
            if s.name == self.struct_name {
                // Find property
                for p in &s.properties {
                    if p.name == self.prop {
                        return p;
                    }
                }
            }
        }
        panic!("Cannot find struct {} of id", self.struct_name);
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Struct {
    pub name: String,
    pub id: Vec<Id>,
    pub doc: String,
    pub accessor: Accessors,
    pub properties: Vec<Property>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Property {
    /// The name of this property (in PascalCase) which can be called from rust when generated.
    pub name: String,
    /// The rust declaration type.
    #[serde(rename = "type")]
    pub type_s: String,
    pub doc: Option<String>,
    pub get: Option<bool>,
    pub set: Option<bool>,
    #[serde(default = "get_false")]
    pub opt: bool,
    #[serde(rename = "mod")]
    pub modifier: Option<String>,
    pub key: Option<String>,
}

impl Property {
    pub fn get_get(&self, struc: &Struct) -> bool {
        self.get.unwrap_or_else(|| struc.accessor.get)
    }
    pub fn get_set(&self, struc: &Struct) -> bool {
        self.set.unwrap_or_else(|| struc.accessor.set)
    }
    pub fn get_rust_type(&self) -> String {
        let mut res = convert_type(&self.type_s);

        if self.modifier.as_ref().map(|s| s == "array").unwrap_or(false) {
            res = format!("Vec<{}>", res);
        } else if self.modifier.as_ref().map(|s| s == "map").unwrap_or(false) {
            let key = self.key.as_ref().expect("Specified map without key");
            res = format!("Map<{}, {}>", key, res);
        }
        if self.opt {
            res = format!("Option<{}>", res);
        }
        res
    }
}

pub enum PropId<'a> {
    Prop(&'a Property),
    Id(&'a Id),
}

impl<'a> PropId<'a> {
    pub fn get_attr_name(&self, struc: &Struct) -> String {
        match *self {
            PropId::Prop(p) => to_snake_case(&p.name),
            PropId::Id(id) => if struc.name == id.struct_name {
                to_snake_case(&id.prop)
            } else {
                format!("{}_{}", to_snake_case(&id.struct_name),
                    to_snake_case(&id.prop))
            }
        }
    }

    pub fn get_doc(&self) -> Option<&str> {
        match *self {
            PropId::Prop(p) => p.doc.as_ref().map(|s| s.as_str()),
            PropId::Id(_) => None,
        }
    }

    pub fn get_rust_type(&self, structs: &[Struct]) -> String {
        match *self {
            PropId::Prop(p) => p.get_rust_type(),
            PropId::Id(id) => id.find_property(structs).get_rust_type(),
        }
    }
}

impl<'a> From<&'a Property> for PropId<'a> {
    fn from(p: &'a Property) -> Self {
        PropId::Prop(p)
    }
}

impl<'a> From<&'a Id> for PropId<'a> {
    fn from(p: &'a Id) -> Self {
        PropId::Id(p)
    }
}

pub fn convert_type(t: &str) -> String {
    if t == "str" {
        String::from("String")
    } else if t == "DateTime" {
        String::from("DateTime<Utc>")
    } else if t == "TimeSpan" {
        String::from("Duration")
    } else {
        t.into()
    }
}
