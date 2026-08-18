// Concise TL schema testing all td-gen codegen features

---types---

// Primitive & built-in type declarations
double ? = Double;
string ? = String;
int32 = Int32;
vector {t:Type} # [ t ] = Vector t;

//@class Item @description Represents a polymorphic item hierarchy
//@description Unit variant item with no fields
itemEmpty = Item;
//@description Data variant item with fields
//@id Item identifier
//@details Additional item details;
//-may be null if omitted
itemData#1234abcd id:int32 details:string = Item;

// Multi-variant enum without unit default (manual Default impl),
// all primitives, int64 attributes, and nested vectors
user id:int32 token:int53 score:int64 ratio:double name:string payload:bytes active:bool = User;
userGroup ids:vector<int64> matrix:vector<vector<string>> = User;

// Recursive type (SCC cycle -> Box<enums::Tree>) with unit default
leaf = Tree;
node value:int32 left:Tree right:Tree = Tree;

// Reference to enum, unresolved custom type, optional field with "pass null"
//@description An entity referencing other types
//@filter Filter expression; pass null for all
entity#5678ef01 target:User extra:CustomType<int32> parent:Tree filter:string = Entity;

// Rust keyword escaping on struct and fields
loop#9abcde for:bool match:int32 = Loop;

---functions---

// Function with fields returning an enum, with int64 serde and optional field
//@description Fetches an entity by ID
//@id Entity ID
//@query Optional search query;
//-may be null
fetchEntity id:int64 query:string = Entity;

// Function without fields returning a primitive
ping = Bool;

// Function with keyword name, keyword field, foreign type, returning a primitive
match type:int32 custom:CustomType = Int32;
