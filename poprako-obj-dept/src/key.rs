pub struct ObjKey {
    //
    pub id: String,
    pub version: u32,
}

pub struct ObjKeyRef<'a> {
    //
    pub id: &'a str,
    pub version: u32,
}
