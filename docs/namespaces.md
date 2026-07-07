# Module / Namespace / Type Name Design Draft

これは仮メモ。
型検査を本格実装する前に、名前解決と module の責務を固めるための設計案。

## 目的

型注釈を parse できても、型名がどの定義を指すかが決まっていないと後段が不安定になる。

例:

```rex
fn main(enemy: Enemy):
    print enemy
```

この `Enemy` は文字列ではなく、最終的には「どこかの module に属する型定義 ID」を指す必要がある。

そのため、TypeCheck より前に以下を分ける。

- 名前解決: ソース上の名前を symbol ID に対応づける
- 型検査: 解決済み ID を使って型の整合性を見る

## 基本方針

AST は source に書かれた構文をなるべく保存する。

NameResolve は名前を ID に対応づける。
ここで value namespace と type namespace を分ける。

TypeCheck は名前 lookup をしない。
TypeCheck は NameResolve 済みの TypeRef を見る。

Types / TypeTable は型定義と型 ID の実体を持つ。
文字列名から探す責務は持たない。

## Namespace

Rex は少なくとも以下の namespace を分ける。

```text
value namespace:
  local
  function
  const / global value

type namespace:
  struct-like type
  enum-like type
  trait-like type
  core type

module namespace:
  module
  imported module alias
```

同じ module 内で、value と type は同名を許すかは未決定。

候補:

```rex
Enemy       # type
Enemy::new # type-associated value
enemy       # local value
```

この形を自然に扱うなら、value namespace と type namespace は分ける方がよい。

## Path Syntax

暫定方針:

```text
::  compile-time path access
.   runtime value member access
```

`::` は module / type / namespace のように、まだ runtime value そのものを経由していない名前をたどるために使う。
`.` は runtime value の field / method に触るために使う。

例:

```rex
Direction::Up
Enemy::new()
std::io::File

enemy.health
enemy.take_damage(10)
```

読みやすさの意図:

- `Direction::Up` は斜め読みで enum/type 側の名前だと分かりやすい
- `Enemy::new()` は type-associated value だと分かりやすい
- `enemy.health` は runtime value の field access だと分かりやすい
- `.` only より記号は増えるが、型/module/value の境界が目で追いやすい

最小ルール:

```text
A::B:
  A は module / type / namespace のような compile-time name
  B は module / type / associated value のどれか

a.b:
  a は runtime value
  b は field または method
```

最初は以下を後回しにする。

- UFCS
- trait method lookup
- method を値として取り出す構文
- import alias / re-export
- 高度な同名解決

## Module

module は item の owner になる。

```text
ModuleId
  owns ItemId
  owns value namespace
  owns type namespace
  owns module namespace
```

最初は root module だけでよい。
ただし symbol table は最初から module 単位の namespace を持つ。

つまり、実装初期でもこう考える。

```text
root module
  value namespace:
    main -> FunctionId

  type namespace:
    int -> CoreTypeId
    float -> CoreTypeId
    Enemy -> TypeSymbolId
```

将来ファイル module や明示 module 構文が入っても、root module の特殊ケースを広げるだけにする。

## Core Types

`int`, `float`, `bool`, `String`, `unit` などは core module または prelude から見える型とする。

言語表面では普通の型名として扱う。
ただし compiler-known な型として印を持つ。

重要:

- `int` は TypeCheck が文字列で特別扱いしない
- `int` は type namespace に登録された型 symbol として解決される
- TypeCheck は解決後の ID が core int を指すかを見る

概念的には:

```text
core::int -> TypeSymbolId -> TypeDefId
```

source で単に `int` と書けるかは prelude/import の責任。

## TypeRef Resolution

AST:

```text
TypeRef::Named("Enemy")
TypeRef::Generic(base: "Vec", args: ["Enemy"])
TypeRef::Function(...)
```

NameResolve 後:

```text
AstTypeRefId -> ResolvedTypeRef

ResolvedTypeRef::Type(TypeSymbolId)
ResolvedTypeRef::Generic {
  base: TypeSymbolId,
  args: Vec<ResolvedTypeRefId>
}
ResolvedTypeRef::Function { ... }
ResolvedTypeRef::Error
```

今の AST には `TypeRefId` がない。
将来 TypeRef の解決結果を持つなら、TypeRef も arena 化するか、AST node の位置を key にできる形にする必要がある。

短期案:

- `ResolvedTypes` を作る
- 関数 param / return / local annotation ごとに解決結果を持つ

例:

```text
function_param_types: Vec<Option<ResolvedTypeRef>>
function_return_types: Vec<Option<ResolvedTypeRef>>
local_annotation_types: Vec<Option<ResolvedTypeRef>>
```

中期案:

- AST の TypeRef を arena 化して `TypeRefId` を導入する
- `ResolvedTypeRefs { refs: Vec<Option<ResolvedTypeRef>> }` を持つ

## Type Definitions

具体構文はまだ固定しない。

ただし、どの構文になっても AST / symbol table 上は以下のような概念を持つ。

```text
TypeSymbol
  name: Ident
  owner: ModuleId
  source: TypeSource

TypeSource:
  Core(CoreType)
  Item(ItemId)
```

`struct`, `enum`, `trait`, `type alias` などは将来の具体的な item kind。
先に type namespace の入れ物を作っておく。

## Name Resolution Order

型名 `Enemy` を解決するとき:

1. 現在の module の type namespace
2. 明示 import
3. prelude / core
4. 見つからなければ `resolve.undefined_type`

今は unqualified name のみを実装対象にする。
ただし、将来の parser/name resolver は `::` を namespace access として扱える形にする。

実装メモ:

- lexer に `::` token が必要
- parser の type path / expr path は `::` を読める必要がある
- `A::B` は AST 上では path として保存し、NameResolve で namespace をたどる
- `A.B` は value access として別 AST にする

## TypeCheck の責務

TypeCheck は以下をしない。

- 文字列名から型を探す
- module import を見る
- prelude を見る

TypeCheck は以下をする。

- resolved type ref から TypeId を作る
- expression に TypeId を貼る
- initializer / return / call / operator の整合性を見る
- overload / method / coercion を解決するならここで扱う

## 実装順

1. AST/parser は型注釈を `TypeRef` として読む
2. symbol table を module owner 前提に整理する
3. value namespace と type namespace を分ける
4. core type symbols を type namespace に登録する
5. type annotation の NameResolve を追加する
6. TypeTable / TypeCheck を追加する

現時点では 1 までで止める。
2 以降が入るまで、本格 TypeCheck は実装しない。

## 未決定

- module はファイル単位か、明示構文を持つか
- import/use の構文
- value と type の同名を許すか
- core/prelude の見え方
- type alias を持つか
- generic type argument の記法は `Vec[T]` で固定か
- `Self` の扱い
