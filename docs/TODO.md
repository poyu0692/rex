# TODO

Rex の次の実装方針。
今の lexer/parser/name resolver/HIR は、縦に通すための prototype として扱う。
ここから先は module / namespace / symbol table を先に固める。

## Links

- [namespaces.md](namespaces.md)
  - module / namespace / type name / `::` と `.` の設計メモ
- [pipeline.md](pipeline.md)
  - compiler pipeline の大枠メモ
- [syntax_idea_sample](syntax_idea_sample)
  - Rex syntax のラフなサンプル

## Current State

できている:

- lexer
- parser
- AST arena
- 関数 item / local binding / print / arithmetic expression
- 関数引数、戻り値、local の型注釈 parse
- `TypeRef::Named`
- `TypeRef::Generic`
- prototype name resolver
- prototype HIR lowering

まだ土台として足りない:

- module-aware symbol table
- value namespace / type namespace / module namespace の分離
- core/prelude type symbols
- type annotation の name resolution
- `ResolvedTypeRef`
- HIR の新 resolver への追従
- TypeTable / TypeCheck

## Ground Rules

- TypeCheck はまだ本格実装しない。
- TypeCheck に文字列 name lookup を持ち込まない。
- `Types` / `TypeTable` は文字列名から型を探さない。
- 名前 lookup は NameResolve / symbol table の責務にする。
- HIR lowering は、module-aware NameResolve ができてから作り直す。
- 既存 HIR は prototype として残してよいが、今後の土台として無理に伸ばさない。

## Next Implementation Order

### 1. Symbol Table を作り直す

`name_resolver.rs` を module-aware な構造に寄せる。

最初は root module だけでよい。
ただし構造は最初から module ごとの namespace を持つ。

```text
Symbols
  modules: Vec<ModuleSymbol>
  functions: Vec<FunctionSymbol>
  locals: Vec<LocalSymbol>
  types: Vec<TypeSymbol>

ModuleSymbol
  ast: ast::ModuleId
  parent: Option<ModuleSymbolId>
  values: Namespace<ValueSymbol>
  types: Namespace<TypeSymbolId>
  modules: Namespace<ModuleSymbolId>
```

最初の goal:

- root module symbol を作る
- top-level functions を root module の value namespace に登録する
- duplicate function は root module value namespace の重複として診断する
- local resolution は今の動作を維持する

### 2. Type Namespace の箱を入れる

まだ型定義構文は固定しない。
ただし type namespace の symbol は先に用意する。

```text
TypeSymbol
  name: Ident
  owner: ModuleSymbolId
  source: TypeSource

TypeSource
  Core(CoreType)
  Item(ItemId)
```

最初の goal:

- root module または prelude/core namespace に core type symbols を登録する
- `int`, `float`, `bool`, `String`, `unit` を type namespace から解決できる形にする
- TypeCheck はまだ作らない

### 3. Path Syntax の AST を決める

`::` は compile-time path access。
`.` は runtime value member access。

最初の goal:

- lexer に `::` token を追加する
- type annotation 内で `std::io::File` のような type path を parse できるようにする
- expr 側の `Enemy::new()` は後回しでもよい

AST 案:

```text
Path
  segments: Vec<Ident>

TypeRef::Path(Path)
```

今の `TypeRef::Named(Ident)` は、単一 segment path として扱える。

### 4. TypeRef の NameResolve を追加する

`ResolvedTypeRef` を NameResolve の出力に入れる。

短期案:

```text
ResolvedTypes
  function_param_types
  function_return_types
  local_annotation_types
```

中期案:

```text
TypeRefId
ResolvedTypeRefs
  refs: Vec<Option<ResolvedTypeRef>>
```

`ResolvedTypeRef` 案:

```text
ResolvedTypeRef::Type(TypeSymbolId)
ResolvedTypeRef::Generic {
  base: TypeSymbolId,
  args: Vec<ResolvedTypeRef>
}
ResolvedTypeRef::Function { ... }
ResolvedTypeRef::Error
```

最初の goal:

- `fn main(x: int):` の `int` を core type symbol に解決する
- unknown type は `resolve.undefined_type` で診断する
- generic は parse 済みでも、解決は最小限でよい

### 5. HIR Lowering を新 NameResolve に合わせる

module-aware resolver ができてから HIR を見直す。

最初の goal:

- HIR が `LocalId` / `FunctionId` に加えて、新 resolver の symbol IDs を前提にする
- type annotation 自体は HIR に落とすか、別 table に残すか決める
- lowering が未解決文字列を見ないようにする

### 6. TypeTable / TypeCheck

ここまで終わってから TypeCheck に戻る。

最初の goal:

- `ResolvedTypeRef -> TypeId`
- `HirExprId -> TypeId`
- initializer mismatch
- return mismatch
- numeric binary operator

## Do Not Do Yet

- TypeCheck の本格実装
- method lookup
- trait lookup
- overload
- coercion
- ownership check
- UFCS
- import alias
- re-export
- multi-file module loading

## Open Questions

- module はファイル単位か、明示構文を持つか
- core/prelude はどう見えるか
- value と type の同名を同じ module 内で許すか
- type alias を持つか
- `Self` はいつ解決するか
- `Vec[T]` の generic 記法を固定するか
- expr 側の `Enemy::new()` をいつ parse するか
