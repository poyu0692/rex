AST:
  source に書かれた構文をなるべく保存する。
  意味解釈しない。

NameResolve:
  名前を id に対応づける。
  型・所有権・mutability の判断はしない。

HIR:
  名前解決済みの意味木。
  parser 都合の構文は少し落とす。
  型はまだ未確定。
  ownership に必要な情報は消さない。

TypeCheck:
  HirExprId -> TypeId を作る。
  overload/method/coercion があるならここで解決。

OwnershipCheck:
  TypeCheck 後の HIR + TypeTable を見る。
  move/copy/read/ref/own/let/var のルールを検査する。

MIR:
  制御フロー、temporaries、drop、代入、branch を明示する。
  実行・最適化・codegen 用。