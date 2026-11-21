//! Autodesk FBX (binary, version 7.4) export of the whole model in 3D,
//! for visualisation tools (Twinmotion, Lumion, Enscape, D5, 3ds Max,
//! Cinema 4D, Blender, Unreal, Unity).
//!
//! Every element as it shows in 3D is one mesh, its layers (a wall's
//! plaster, masonry and insulation, a window's frame and glass) one
//! material each. The meshes hang in a tree: the building, then each
//! storey, then what the elements are (walls, slabs, windows…); elements
//! of no storey (the terrain, the site) under "Ostatné". Each material
//! carries the colour, gloss and transparency of the CreaBIM material;
//! the meshes carry flat normals and box-mapped UVs in metres, so a
//! texture of 1 × 1 m tiles at its true size.
//!
//! The file is Z-up in centimetres (FBX's own unit), which the tools read
//! from its global settings.

use creabim_core::{ElementId, ElementKind, Model, OpeningKind};
use std::collections::HashMap;

/// The file's units: centimetres per model metre.
const CM: f64 = 100.0;

/// The whole model as a binary FBX file.
pub fn export_fbx(model: &Model, title: &str) -> Vec<u8> {
    let scene = creabim_geom::scene::drawn_scene(model);
    // each element's parts, in the order they come
    let mut order: Vec<ElementId> = vec![];
    let mut parts: HashMap<ElementId, Vec<&creabim_geom::scene::SceneItem>> = HashMap::new();
    for item in &scene {
        if item.mesh.indices.is_empty() || model.get(item.id).is_none() {
            continue;
        }
        parts
            .entry(item.id)
            .or_insert_with(|| {
                order.push(item.id);
                vec![]
            })
            .push(item);
    }

    let mut ids = IdGen(1_000_000);
    let mut objects: Vec<Node> = vec![];
    let mut links: Vec<(i64, i64)> = vec![];
    let (mut n_models, mut n_geoms, mut n_attrs) = (0usize, 0usize, 0usize);

    // the materials, one per look
    let mut materials: Vec<(String, [f32; 3], [f32; 3], i64)> = vec![];
    let mut material_at = |name: String,
                           color: [f32; 3],
                           surface: [f32; 3],
                           objects: &mut Vec<Node>,
                           ids: &mut IdGen|
     -> i64 {
        let key = |c: [f32; 3]| c.map(|v| (v * 255.0).round() as i32);
        if let Some(m) = materials
            .iter()
            .find(|m| m.0 == name && key(m.1) == key(color) && key(m.2) == key(surface))
        {
            return m.3;
        }
        // the same name in another colour gets a number
        let same = materials
            .iter()
            .filter(|m| m.0 == name || m.0.starts_with(&format!("{name} (")))
            .count();
        let shown = if same == 0 {
            name.clone()
        } else {
            format!("{name} ({})", same + 1)
        };
        let id = ids.next();
        objects.push(material_node(id, &shown, color, surface));
        materials.push((name, color, surface, id));
        id
    };

    // the tree: building, storeys, kinds
    let root = ids.next();
    let title = if title.trim().is_empty() {
        "CreaBIM"
    } else {
        title
    };
    objects.push(null_model(root, title));
    let attr = ids.next();
    objects.push(null_attribute(attr));
    links.push((attr, root));
    links.push((root, 0));
    n_models += 1;
    n_attrs += 1;
    let storeys = creabim_core::levels::sorted(model);
    let mut groups: HashMap<(Option<ElementId>, &'static str), i64> = HashMap::new();
    let mut storey_node: HashMap<Option<ElementId>, i64> = HashMap::new();

    for id in &order {
        let e = model.get(*id).expect("an element of the model");
        let level = level_of(model, e).filter(|l| storeys.iter().any(|s| s.0 == *l));
        let storey = *storey_node.entry(level).or_insert_with(|| {
            let name = level
                .and_then(|l| storeys.iter().find(|s| s.0 == l))
                .map_or("Ostatné".to_string(), |s| s.1.clone());
            let n = ids.next();
            objects.push(null_model(n, &name));
            let a = ids.next();
            objects.push(null_attribute(a));
            links.push((a, n));
            links.push((n, root));
            n_models += 1;
            n_attrs += 1;
            n
        });
        let cat = category(e);
        let group = *groups.entry((level, cat)).or_insert_with(|| {
            let n = ids.next();
            objects.push(null_model(n, cat));
            let a = ids.next();
            objects.push(null_attribute(a));
            links.push((a, n));
            links.push((n, storey));
            n_models += 1;
            n_attrs += 1;
            n
        });

        // the element's mesh: one material per part
        let mut mats: Vec<i64> = vec![];
        let mut weld: HashMap<(i64, i64, i64), i32> = HashMap::new();
        let mut verts: Vec<[f64; 3]> = vec![];
        let mut poly: Vec<i32> = vec![];
        let mut normals: Vec<f64> = vec![];
        let mut uvs: Vec<f64> = vec![];
        let mut poly_mat: Vec<i32> = vec![];
        for item in &parts[id] {
            let (name, color, surface) = look(model, e, item);
            let m = material_at(name, color, surface, &mut objects, &mut ids);
            let mi = match mats.iter().position(|x| *x == m) {
                Some(i) => i,
                None => {
                    mats.push(m);
                    mats.len() - 1
                }
            } as i32;
            let mesh = &item.mesh;
            for t in mesh.indices.chunks_exact(3) {
                let p = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize].map(f64::from));
                let n = face_normal(p);
                let Some(n) = n else {
                    continue; // a triangle with no area
                };
                for (k, q) in p.iter().enumerate() {
                    let key = (
                        (q[0] * 1e5).round() as i64,
                        (q[1] * 1e5).round() as i64,
                        (q[2] * 1e5).round() as i64,
                    );
                    let v = *weld.entry(key).or_insert_with(|| {
                        verts.push(*q);
                        verts.len() as i32 - 1
                    });
                    poly.push(if k == 2 { !v } else { v });
                    normals.extend(n);
                    uvs.extend(box_uv(*q, n));
                }
                poly_mat.push(mi);
            }
        }
        if poly.is_empty() {
            continue;
        }
        let model_id = ids.next();
        let geom_id = ids.next();
        let name = if e.name.trim().is_empty() {
            cat.to_string()
        } else {
            e.name.clone()
        };
        objects.push(mesh_model(model_id, &name, e.id));
        objects.push(geometry(
            geom_id, &name, &verts, &poly, &normals, &uvs, &poly_mat,
        ));
        n_models += 1;
        n_geoms += 1;
        links.push((model_id, group));
        links.push((geom_id, model_id));
        for m in &mats {
            links.push((*m, model_id));
        }
    }

    let mut top = vec![
        header(),
        Node::new("FileId").raw(FILE_ID.to_vec()),
        Node::new("CreationTime").string(TIME_ID),
        Node::new("Creator").string(&creator()),
        global_settings(),
        documents(),
        Node::new("References"),
        definitions(n_models, n_geoms, materials.len(), n_attrs),
    ];
    top.push(Node::new("Objects").children(objects));
    top.push(
        Node::new("Connections").children(
            links
                .iter()
                .map(|(c, p)| Node::new("C").string("OO").long(*c).long(*p))
                .collect(),
        ),
    );
    top.push(Node::new("Takes").children(vec![Node::new("Current").string("")]));
    write_file(top)
}

/// What an element is, for the tree: the group it hangs in.
fn category(e: &creabim_core::Element) -> &'static str {
    match &e.kind {
        ElementKind::Wall(_) => "Steny",
        ElementKind::Slab(_) => "Dosky",
        ElementKind::Opening(o) => match o.kind {
            OpeningKind::Window => "Okná",
            OpeningKind::Door => "Dvere",
            OpeningKind::Hole => "Otvory",
        },
        ElementKind::Roof(_) => "Strechy",
        ElementKind::Column(_) => "Stĺpy",
        ElementKind::Beam(_) => "Nosníky",
        ElementKind::Stair(_) => "Schodiská",
        ElementKind::Railing(_) => "Zábradlia",
        ElementKind::Footing(_) => "Základy",
        ElementKind::Ceiling(_) => "Podhľady",
        ElementKind::Object(_) => "Objekty a zariaďovacie predmety",
        ElementKind::Pipe(_) | ElementKind::Valve(_) => "Rozvody",
        ElementKind::Site(_) => "Terén",
        ElementKind::Part(_) => "Časti",
        ElementKind::Shaft(_) => "Šachty",
        _ => "Ostatné",
    }
}

fn level_of(model: &Model, e: &creabim_core::Element) -> Option<ElementId> {
    match &e.kind {
        ElementKind::Opening(o) => model
            .get(o.host)
            .and_then(|h| creabim_core::levels::level_of(&h.kind)),
        k => creabim_core::levels::level_of(k),
    }
}

/// A part's material: its name, colour (sRGB) and roughness, metallic,
/// opacity.
fn look(
    model: &Model,
    e: &creabim_core::Element,
    item: &creabim_geom::scene::SceneItem,
) -> (String, [f32; 3], [f32; 3]) {
    if let Some(name) = item.finish {
        return (name.to_string(), item.color, item.surface);
    }
    let named = |id: ElementId| match model.get(id) {
        Some(m) if matches!(m.kind, ElementKind::Material(_)) => Some(m.name.clone()),
        _ => None,
    };
    if let Some(name) = item.layer.and_then(|l| l.material).and_then(named) {
        return (name, item.color, item.surface);
    }
    if let Some((m, _)) = creabim_core::material_of(model, e) {
        return (m.name.clone(), item.color, item.surface);
    }
    (category(e).to_string(), item.color, item.surface)
}

fn face_normal(p: [[f64; 3]; 3]) -> Option<[f64; 3]> {
    let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
    let v = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    (l > 1e-12).then(|| n.map(|c| c / l))
}

/// Box mapping: a texture metre is a model metre, on the plane the face
/// faces most.
fn box_uv(p: [f64; 3], n: [f64; 3]) -> [f64; 2] {
    let a = n.map(f64::abs);
    if a[2] >= a[0] && a[2] >= a[1] {
        [p[0], p[1]]
    } else if a[0] >= a[1] {
        [p[1] * n[0].signum(), p[2]]
    } else {
        [-p[0] * n[1].signum(), p[2]]
    }
}

struct IdGen(i64);

impl IdGen {
    fn next(&mut self) -> i64 {
        self.0 += 1;
        self.0
    }
}

// ---- the FBX objects ----

fn class_name(name: &str, class: &str) -> String {
    // FBX keeps "name\0\x01Class"; a NUL in the name would cut it
    format!("{}\u{0}\u{1}{class}", name.replace('\u{0}', ""))
}

fn p(name: &str, t1: &str, t2: &str, flags: &str) -> Node {
    Node::new("P")
        .string(name)
        .string(t1)
        .string(t2)
        .string(flags)
}

fn null_model(id: i64, name: &str) -> Node {
    Node::new("Model")
        .long(id)
        .string(&class_name(name, "Model"))
        .string("Null")
        .children(vec![
            Node::new("Version").int(232),
            Node::new("Properties70").children(vec![
                p("InheritType", "enum", "", "").int(1),
                p("DefaultAttributeIndex", "int", "Integer", "").int(0),
            ]),
            Node::new("Shading").bool(true),
            Node::new("Culling").string("CullingOff"),
        ])
}

fn null_attribute(id: i64) -> Node {
    Node::new("NodeAttribute")
        .long(id)
        .string(&class_name("", "NodeAttribute"))
        .string("Null")
        .children(vec![
            Node::new("TypeFlags").string("Null"),
            Node::new("Properties70").children(vec![p("Look", "enum", "", "").int(0)]),
        ])
}

fn mesh_model(id: i64, name: &str, creabim: ElementId) -> Node {
    Node::new("Model")
        .long(id)
        .string(&class_name(name, "Model"))
        .string("Mesh")
        .children(vec![
            Node::new("Version").int(232),
            Node::new("Properties70").children(vec![
                p("InheritType", "enum", "", "").int(1),
                p("DefaultAttributeIndex", "int", "Integer", "").int(0),
                // the element's id, kept as a user property
                p("creabim_id", "KString", "", "U").string(&creabim.0.to_string()),
            ]),
            Node::new("Shading").bool(true),
            Node::new("Culling").string("CullingOff"),
        ])
}

fn geometry(
    id: i64,
    name: &str,
    verts: &[[f64; 3]],
    poly: &[i32],
    normals: &[f64],
    uvs: &[f64],
    poly_mat: &[i32],
) -> Node {
    let v: Vec<f64> = verts.iter().flat_map(|q| q.map(|c| c * CM)).collect();
    let uv_index: Vec<i32> = (0..poly.len() as i32).collect();
    let all_same = poly_mat.iter().all(|m| *m == poly_mat[0]);
    let (mapping, mats) = if all_same {
        ("AllSame", vec![poly_mat[0]])
    } else {
        ("ByPolygon", poly_mat.to_vec())
    };
    let layer = |t: &str| {
        Node::new("LayerElement").children(vec![
            Node::new("Type").string(t),
            Node::new("TypedIndex").int(0),
        ])
    };
    Node::new("Geometry")
        .long(id)
        .string(&class_name(name, "Geometry"))
        .string("Mesh")
        .children(vec![
            Node::new("Properties70"),
            Node::new("GeometryVersion").int(124),
            Node::new("Vertices").doubles(&v),
            Node::new("PolygonVertexIndex").ints(poly),
            Node::new("LayerElementNormal").int(0).children(vec![
                Node::new("Version").int(101),
                Node::new("Name").string(""),
                Node::new("MappingInformationType").string("ByPolygonVertex"),
                Node::new("ReferenceInformationType").string("Direct"),
                Node::new("Normals").doubles(normals),
            ]),
            Node::new("LayerElementUV").int(0).children(vec![
                Node::new("Version").int(101),
                Node::new("Name").string("UVMap"),
                Node::new("MappingInformationType").string("ByPolygonVertex"),
                Node::new("ReferenceInformationType").string("IndexToDirect"),
                Node::new("UV").doubles(uvs),
                Node::new("UVIndex").ints(&uv_index),
            ]),
            Node::new("LayerElementMaterial").int(0).children(vec![
                Node::new("Version").int(101),
                Node::new("Name").string(""),
                Node::new("MappingInformationType").string(mapping),
                Node::new("ReferenceInformationType").string("IndexToDirect"),
                Node::new("Materials").ints(&mats),
            ]),
            Node::new("Layer").int(0).children(vec![
                Node::new("Version").int(100),
                layer("LayerElementNormal"),
                layer("LayerElementUV"),
                layer("LayerElementMaterial"),
            ]),
        ])
}

/// sRGB to linear light, as renderers take colours.
fn linear(c: f32) -> f64 {
    let c = c as f64;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn material_node(id: i64, name: &str, color: [f32; 3], surface: [f32; 3]) -> Node {
    let [roughness, metallic, opacity] = surface.map(|v| v as f64);
    let c = color.map(linear);
    let shine = ((1.0 - roughness).clamp(0.0, 1.0) * 100.0).max(2.0);
    let spec = 0.04 + 0.5 * (1.0 - roughness).clamp(0.0, 1.0) + 0.4 * metallic;
    let rgb = |n: Node, v: [f64; 3]| n.double(v[0]).double(v[1]).double(v[2]);
    Node::new("Material")
        .long(id)
        .string(&class_name(name, "Material"))
        .string("")
        .children(vec![
            Node::new("Version").int(102),
            Node::new("ShadingModel").string("phong"),
            Node::new("MultiLayer").int(0),
            Node::new("Properties70").children(vec![
                rgb(p("DiffuseColor", "Color", "", "A"), c),
                p("DiffuseFactor", "Number", "", "A").double(1.0),
                rgb(p("SpecularColor", "Color", "", "A"), [spec.min(1.0); 3]),
                p("SpecularFactor", "Number", "", "A").double(1.0),
                p("Shininess", "Number", "", "A").double(shine),
                p("ShininessExponent", "Number", "", "A").double(shine),
                rgb(p("TransparentColor", "Color", "", "A"), [1.0; 3]),
                p("TransparencyFactor", "Number", "", "A").double(1.0 - opacity),
                p("Opacity", "Number", "", "A").double(opacity),
                rgb(p("Diffuse", "Vector3D", "Vector", ""), c),
                rgb(p("Specular", "Vector3D", "Vector", ""), [spec.min(1.0); 3]),
                p("Shininess", "double", "Number", "").double(shine),
                p("Reflectivity", "double", "Number", "").double(metallic),
            ]),
        ])
}

const FILE_ID: [u8; 16] = [
    0x28, 0xb3, 0x2a, 0xeb, 0xb6, 0x24, 0xcc, 0xc2, 0xbf, 0xc8, 0xb0, 0x2a, 0xa9, 0x2b, 0xfc, 0xf1,
];
const TIME_ID: &str = "1970-01-01 10:00:00:000";
const FOOT_ID: [u8; 16] = [
    0xfa, 0xbc, 0xab, 0x09, 0xd0, 0xc8, 0xd4, 0x66, 0xb1, 0x76, 0xfb, 0x83, 0x1c, 0xf7, 0x26, 0x7e,
];
const VERSION: u32 = 7400;

fn creator() -> String {
    format!("CreaBIM {}", env!("CARGO_PKG_VERSION"))
}

fn header() -> Node {
    let stamp = Node::new("CreationTimeStamp").children(vec![
        Node::new("Version").int(1000),
        Node::new("Year").int(1970),
        Node::new("Month").int(1),
        Node::new("Day").int(1),
        Node::new("Hour").int(10),
        Node::new("Minute").int(0),
        Node::new("Second").int(0),
        Node::new("Millisecond").int(0),
    ]);
    let info = Node::new("SceneInfo")
        .string(&class_name("GlobalInfo", "SceneInfo"))
        .string("UserData")
        .children(vec![
            Node::new("Type").string("UserData"),
            Node::new("Version").int(100),
            Node::new("MetaData").children(vec![
                Node::new("Version").int(100),
                Node::new("Title").string(""),
                Node::new("Subject").string(""),
                Node::new("Author").string(""),
                Node::new("Keywords").string(""),
                Node::new("Revision").string(""),
                Node::new("Comment").string(""),
            ]),
            Node::new("Properties70").children(vec![
                p("DocumentUrl", "KString", "Url", "").string("CreaBIM.fbx"),
                p("SrcDocumentUrl", "KString", "Url", "").string("CreaBIM.fbx"),
                p("Original", "Compound", "", ""),
                p("Original|ApplicationVendor", "KString", "", "").string("CreaBIM"),
                p("Original|ApplicationName", "KString", "", "").string("CreaBIM"),
                p("Original|ApplicationVersion", "KString", "", "")
                    .string(env!("CARGO_PKG_VERSION")),
                p("Original|DateTime_GMT", "DateTime", "", "").string("01/01/1970 00:00:00.000"),
                p("Original|FileName", "KString", "", "").string("CreaBIM.fbx"),
                p("LastSaved", "Compound", "", ""),
                p("LastSaved|ApplicationVendor", "KString", "", "").string("CreaBIM"),
                p("LastSaved|ApplicationName", "KString", "", "").string("CreaBIM"),
                p("LastSaved|ApplicationVersion", "KString", "", "")
                    .string(env!("CARGO_PKG_VERSION")),
                p("LastSaved|DateTime_GMT", "DateTime", "", "").string("01/01/1970 00:00:00.000"),
            ]),
        ]);
    Node::new("FBXHeaderExtension").children(vec![
        Node::new("FBXHeaderVersion").int(1003),
        Node::new("FBXVersion").int(VERSION as i32),
        Node::new("EncryptionType").int(0),
        stamp,
        Node::new("Creator").string(&creator()),
        info,
    ])
}

fn global_settings() -> Node {
    Node::new("GlobalSettings").children(vec![
        Node::new("Version").int(1000),
        Node::new("Properties70").children(vec![
            p("UpAxis", "int", "Integer", "").int(2),
            p("UpAxisSign", "int", "Integer", "").int(1),
            p("FrontAxis", "int", "Integer", "").int(1),
            p("FrontAxisSign", "int", "Integer", "").int(-1),
            p("CoordAxis", "int", "Integer", "").int(0),
            p("CoordAxisSign", "int", "Integer", "").int(1),
            p("OriginalUpAxis", "int", "Integer", "").int(2),
            p("OriginalUpAxisSign", "int", "Integer", "").int(1),
            p("UnitScaleFactor", "double", "Number", "").double(1.0),
            p("OriginalUnitScaleFactor", "double", "Number", "").double(1.0),
            p("AmbientColor", "ColorRGB", "Color", "")
                .double(0.0)
                .double(0.0)
                .double(0.0),
            p("DefaultCamera", "KString", "", "").string("Producer Perspective"),
            p("TimeMode", "enum", "", "").int(11),
            p("TimeSpanStart", "KTime", "Time", "").long(0),
            p("TimeSpanStop", "KTime", "Time", "").long(46_186_158_000),
            p("CustomFrameRate", "double", "Number", "").double(24.0),
        ]),
    ])
}

fn documents() -> Node {
    Node::new("Documents").children(vec![
        Node::new("Count").int(1),
        Node::new("Document")
            .long(1)
            .string("Scene")
            .string("Scene")
            .children(vec![
                Node::new("Properties70").children(vec![
                    p("SourceObject", "object", "", ""),
                    p("ActiveAnimStackName", "KString", "", "").string(""),
                ]),
                Node::new("RootNode").long(0),
            ]),
    ])
}

fn definitions(models: usize, geoms: usize, mats: usize, attrs: usize) -> Node {
    let t = |name: &str, n: usize| {
        Node::new("ObjectType")
            .string(name)
            .children(vec![Node::new("Count").int(n as i32)])
    };
    let mut kinds = vec![
        Node::new("Version").int(100),
        Node::new("Count").int((1 + models + geoms + mats + attrs) as i32),
        t("GlobalSettings", 1),
        t("Model", models),
        t("NodeAttribute", attrs),
    ];
    if geoms > 0 {
        kinds.push(t("Geometry", geoms));
    }
    if mats > 0 {
        kinds.push(t("Material", mats));
    }
    Node::new("Definitions").children(kinds)
}

// ---- the binary format ----

/// A value of a node's property.
enum Prop {
    Bool(bool),
    Int(i32),
    Long(i64),
    Double(f64),
    Str(Vec<u8>),
    Raw(Vec<u8>),
    Doubles(Vec<f64>),
    Ints(Vec<i32>),
}

/// A node of the file: a name, its properties and the nodes in it.
struct Node {
    name: &'static str,
    props: Vec<Prop>,
    kids: Vec<Node>,
}

impl Node {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            props: vec![],
            kids: vec![],
        }
    }
    fn with(mut self, p: Prop) -> Self {
        self.props.push(p);
        self
    }
    fn bool(self, v: bool) -> Self {
        self.with(Prop::Bool(v))
    }
    fn int(self, v: i32) -> Self {
        self.with(Prop::Int(v))
    }
    fn long(self, v: i64) -> Self {
        self.with(Prop::Long(v))
    }
    fn double(self, v: f64) -> Self {
        self.with(Prop::Double(v))
    }
    fn string(self, v: &str) -> Self {
        self.with(Prop::Str(v.as_bytes().to_vec()))
    }
    fn raw(self, v: Vec<u8>) -> Self {
        self.with(Prop::Raw(v))
    }
    fn doubles(self, v: &[f64]) -> Self {
        self.with(Prop::Doubles(v.to_vec()))
    }
    fn ints(self, v: &[i32]) -> Self {
        self.with(Prop::Ints(v.to_vec()))
    }
    fn children(mut self, kids: Vec<Node>) -> Self {
        self.kids = kids;
        self
    }
}

/// 13 zero bytes end a list of nodes (FBX before 7.5).
const SENTINEL: [u8; 13] = [0; 13];

fn write_props(out: &mut Vec<u8>, props: &[Prop]) {
    fn array(out: &mut Vec<u8>, kind: u8, n: usize, raw: Vec<u8>) {
        out.push(kind);
        out.extend((n as u32).to_le_bytes());
        // long arrays deflated, as FBX writers do
        if raw.len() > 128 {
            use std::io::Write;
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            z.write_all(&raw).expect("in memory");
            let packed = z.finish().expect("in memory");
            out.extend(1u32.to_le_bytes());
            out.extend((packed.len() as u32).to_le_bytes());
            out.extend(packed);
        } else {
            out.extend(0u32.to_le_bytes());
            out.extend((raw.len() as u32).to_le_bytes());
            out.extend(raw);
        }
    }
    for p in props {
        match p {
            Prop::Bool(v) => {
                out.push(b'C');
                out.push(if *v { 1 } else { 0 });
            }
            Prop::Int(v) => {
                out.push(b'I');
                out.extend(v.to_le_bytes());
            }
            Prop::Long(v) => {
                out.push(b'L');
                out.extend(v.to_le_bytes());
            }
            Prop::Double(v) => {
                out.push(b'D');
                out.extend(v.to_le_bytes());
            }
            Prop::Str(s) | Prop::Raw(s) => {
                out.push(if matches!(p, Prop::Str(_)) {
                    b'S'
                } else {
                    b'R'
                });
                out.extend((s.len() as u32).to_le_bytes());
                out.extend(s);
            }
            Prop::Doubles(v) => array(
                out,
                b'd',
                v.len(),
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
            Prop::Ints(v) => array(
                out,
                b'i',
                v.len(),
                v.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ),
        }
    }
}

fn write_node(out: &mut Vec<u8>, n: &Node, last: bool) {
    let start = out.len();
    out.extend([0u8; 12]); // end offset, property count, property bytes
    out.push(n.name.len() as u8);
    out.extend(n.name.as_bytes());
    let props_at = out.len();
    write_props(out, &n.props);
    let props_len = out.len() - props_at;
    if !n.kids.is_empty() {
        let k = n.kids.len();
        for (i, c) in n.kids.iter().enumerate() {
            write_node(out, c, i + 1 == k);
        }
        out.extend(SENTINEL);
    } else if n.props.is_empty() && !last {
        out.extend(SENTINEL);
    }
    let end = out.len() as u32;
    out[start..start + 4].copy_from_slice(&end.to_le_bytes());
    out[start + 4..start + 8].copy_from_slice(&(n.props.len() as u32).to_le_bytes());
    out[start + 8..start + 12].copy_from_slice(&(props_len as u32).to_le_bytes());
}

fn write_file(top: Vec<Node>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend(b"Kaydara FBX Binary  \x00\x1a\x00");
    out.extend(VERSION.to_le_bytes());
    let k = top.len();
    for (i, n) in top.iter().enumerate() {
        write_node(&mut out, n, i + 1 == k);
    }
    out.extend(SENTINEL);
    out.extend(FOOT_ID);
    out.extend([0u8; 4]);
    let pad = match out.len().div_ceil(16) * 16 - out.len() {
        0 => 16,
        n => n,
    };
    out.extend(std::iter::repeat_n(0u8, pad));
    out.extend(VERSION.to_le_bytes());
    out.extend([0u8; 120]);
    out.extend([
        0xf8, 0x5a, 0x8c, 0x6a, 0xde, 0xf5, 0xd9, 0x7e, 0xec, 0xe9, 0x0c, 0xe3, 0x75, 0x8f, 0x29,
        0x0b,
    ]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read the top-level node names and walk every node's offsets.
    fn walk(b: &[u8], mut at: usize, end: usize, names: &mut Vec<String>, depth: usize) -> usize {
        while at + 13 <= end {
            let e = u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize;
            if e == 0 {
                return at + 13;
            }
            let np = u32::from_le_bytes(b[at + 4..at + 8].try_into().unwrap()) as usize;
            let pl = u32::from_le_bytes(b[at + 8..at + 12].try_into().unwrap()) as usize;
            let nl = b[at + 12] as usize;
            let name = String::from_utf8(b[at + 13..at + 13 + nl].to_vec()).unwrap();
            if depth == 0 {
                names.push(name);
            }
            let kids = at + 13 + nl + pl;
            assert!(e <= b.len() && kids <= e, "node offsets fit");
            let _ = np;
            if kids < e {
                walk(b, kids, e, names, depth + 1);
            }
            at = e;
        }
        at
    }

    #[test]
    fn the_sample_house_exports_a_readable_fbx() {
        let m = creabim_core::sample::sample_house();
        let b = export_fbx(&m, "Vzor");
        assert_eq!(&b[..21], b"Kaydara FBX Binary  \x00");
        assert_eq!(u32::from_le_bytes(b[23..27].try_into().unwrap()), 7400);
        let mut names = vec![];
        walk(&b, 27, b.len(), &mut names, 0);
        for n in [
            "FBXHeaderExtension",
            "GlobalSettings",
            "Definitions",
            "Objects",
            "Connections",
        ] {
            assert!(names.iter().any(|x| x == n), "{n} in {names:?}");
        }
        // walls, storeys and materials by name
        let has = |s: &str| b.windows(s.len()).any(|w| w == s.as_bytes());
        assert!(has("Steny\u{0}\u{1}Model"));
        assert!(has("\u{0}\u{1}Material"));
    }
}
