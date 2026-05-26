; Variables
(identifier) @variable

(call_expression
  function: (member_expression
    object: (identifier) @type
    (#any-of? @type
      "Promise" "Array" "Object" "Map" "Set" "WeakMap" "WeakSet" "Date" "Error" "TypeError"
      "RangeError" "SyntaxError" "ReferenceError" "EvalError" "URIError" "RegExp" "Function"
      "Number" "String" "Boolean" "Symbol" "BigInt" "Proxy" "ArrayBuffer" "DataView")))

; Special identifiers
(type_annotation) @type

(type_identifier) @type

(predefined_type) @type.builtin

(type_alias_declaration
  (type_identifier) @type)

(type_alias_declaration
  value: (_
    (type_identifier) @type))

(interface_declaration
  (type_identifier) @type)

(class_declaration
  (type_identifier) @type.class)

(extends_clause
  value: (identifier) @type.class)

(extends_type_clause
  type: (type_identifier) @type)

(implements_clause
  (type_identifier) @type)

; Enables ts-pretty-errors
; The Lsp returns "snippets" of typescript, which are not valid typescript in totality,
; but should still be highlighted
; Highlights object literals by hijacking the statement_block pattern, but only if
; the statement block follows an object literal pattern
(statement_block
  (labeled_statement
    ; highlight the label like a property name
    label: (statement_identifier) @property.name
    body: [
      ; match a terminating expression statement
      (expression_statement
        ; single identifier - treat as a type name
        [
          (identifier) @type.name
          ; object - treat as a property - type pair
          (object
            (pair
              key: (_) @property.name
              value: (_) @type.name))
          ; subscript_expression - treat as an array declaration
          (subscript_expression
            object: (_) @type.name
            index: (_))
          ; templated string - treat each identifier contained as a type name
          (template_string
            (template_substitution
              (identifier) @type.name))
        ])
      ; match a nested statement block
      (statement_block) @nested
    ]))

; Inline type imports: import { type Foo } or import { type Foo as Bar }
(import_specifier
  "type"
  name: (identifier) @type)

(import_specifier
  "type"
  alias: (identifier) @type)

; Full type imports: import type { Foo } or import type { Foo as Bar }
(import_statement
  "type"
  (import_clause
    (named_imports
      (import_specifier
        name: (identifier) @type))))

(import_statement
  "type"
  (import_clause
    (named_imports
      (import_specifier
        alias: (identifier) @type))))

; Import identifier names — uniform color across default, named, alias, namespace
; imports. Heuristics below override for hooks/known functions/types.
(import_clause
  (identifier) @namespace)

(import_specifier
  name: (identifier) @variable.import)

(import_specifier
  alias: (identifier) @variable.import)

(namespace_import
  (identifier) @namespace)

; React hooks and known function-like imports — render as functions (blue).
((import_specifier
  name: (identifier) @function)
  (#match? @function "^use[A-Z]"))

((import_specifier
  alias: (identifier) @function)
  (#match? @function "^use[A-Z]"))

((import_specifier
  name: (identifier) @function)
  (#match? @function "^(memo|forwardRef|createElement|createContext|createRef|render|lazy|reducer|effect|callback)$"))

((import_specifier
  alias: (identifier) @function)
  (#match? @function "^(memo|forwardRef|createElement|createContext|createRef|render|lazy|reducer|effect|callback)$"))

; PascalCase named imports — assume types/components. Same coloring as type
; references; project modules can be styled differently via @type.module.
((import_specifier
  name: (identifier) @type)
  (#match? @type "^[A-Z][a-z]"))

((import_specifier
  alias: (identifier) @type)
  (#match? @type "^[A-Z][a-z]"))

([
  (identifier)
  (shorthand_property_identifier)
  (shorthand_property_identifier_pattern)
] @constant
  (#match? @constant "^_*[A-Z_][A-Z\\d_]*$"))

; Properties
(property_identifier) @property

(shorthand_property_identifier) @property

(shorthand_property_identifier_pattern) @property

(private_property_identifier) @property

; Function and method calls
(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: [
      (property_identifier)
      (private_property_identifier)
    ] @function.method))

(new_expression
  constructor: (identifier) @type)

(nested_type_identifier
  module: (identifier) @type)

; Function and method definitions
(function_expression
  name: (identifier) @function)

(function_declaration
  name: (identifier) @function)

(method_definition
  name: [
    (property_identifier)
    (private_property_identifier)
  ] @function.method)

(method_definition
  name: (property_identifier) @constructor
  (#eq? @constructor "constructor"))

(pair
  key: [
    (property_identifier)
    (private_property_identifier)
  ] @function.method
  value: [
    (function_expression)
    (arrow_function)
  ])

(assignment_expression
  left: (member_expression
    property: [
      (property_identifier)
      (private_property_identifier)
    ] @function.method)
  right: [
    (function_expression)
    (arrow_function)
  ])

(variable_declarator
  name: (identifier) @function
  value: [
    (function_expression)
    (arrow_function)
  ])

(assignment_expression
  left: (identifier) @function
  right: [
    (function_expression)
    (arrow_function)
  ])

(arrow_function) @function

; Parameters
(required_parameter
  (identifier) @variable.parameter)

(required_parameter
  (_
    ([
      (identifier)
      (shorthand_property_identifier_pattern)
    ]) @variable.parameter))

(optional_parameter
  (identifier) @variable.parameter)

(optional_parameter
  (_
    ([
      (identifier)
      (shorthand_property_identifier_pattern)
    ]) @variable.parameter))

(catch_clause
  parameter: (identifier) @variable.parameter)

(index_signature
  name: (identifier) @variable.parameter)

(arrow_function
  parameter: (identifier) @variable.parameter)

(type_predicate
  name: (identifier) @variable.parameter)

; Object-expression shorthand: `{foo}` is a variable reference, not a property
(object
  (shorthand_property_identifier) @variable)

; Object destructure shorthand: `const {foo} = x` declares a variable
(object_pattern
  (shorthand_property_identifier_pattern) @variable)

; PascalCase identifiers used as values (Dialog, AiSimilarityAccessModule, …)
; — color them as functions/components, not as types. Type annotations still
; win because type_identifier captures above are more specific.
((identifier) @function
  (#match? @function "^[A-Z][a-z]"))

((shorthand_property_identifier) @function
  (#match? @function "^[A-Z][a-z]"))

((shorthand_property_identifier_pattern) @function
  (#match? @function "^[A-Z][a-z]"))

; PascalCase variable declarations (React component definitions) — use a
; dedicated scope so the theme can render them as purple semibold like
; WebStorm's semantic highlighting.
((variable_declarator
  name: (identifier) @function.declaration)
  (#match? @function.declaration "^[A-Z][a-z]"))

; React hook tuple destructure: `const [x, setX] = useFoo(...)` — the second
; binding is the setter/dispatcher (function), the first is state (variable).
((variable_declarator
  name: (array_pattern
    (identifier)
    (identifier) @function)
  value: (call_expression
    function: (identifier) @_hook))
  (#match? @_hook "^use"))


; PascalCase identifier used as the object of a member access — module/namespace
; reference, e.g. `AiSimilarityAccessModule.isAllowed(...)`. Render distinct from
; React components passed as values.
((member_expression
  object: (identifier) @type.module)
  (#match? @type.module "^[A-Z][a-z]"))

; Re-assert import-specific captures AFTER the generic PascalCase →
; @function capture so they win on tie-breaking precedence (tree-sitter:
; later pattern wins when patterns match the same byte range).
(import_clause
  (identifier) @namespace)

(namespace_import
  (identifier) @namespace)

((import_specifier
  name: (identifier) @function)
  (#match? @function "^use[A-Z]"))

((import_specifier
  alias: (identifier) @function)
  (#match? @function "^use[A-Z]"))

((import_specifier
  name: (identifier) @function)
  (#match? @function "^(memo|forwardRef|createElement|createContext|createRef|render|lazy|reducer|effect|callback)$"))

((import_specifier
  alias: (identifier) @function)
  (#match? @function "^(memo|forwardRef|createElement|createContext|createRef|render|lazy|reducer|effect|callback)$"))

((import_specifier
  name: (identifier) @type)
  (#match? @type "^[A-Z][a-z]"))

((import_specifier
  alias: (identifier) @type)
  (#match? @type "^[A-Z][a-z]"))

; "Module-shaped" PascalCase imports (ending in Module/Service/Util/Helper/
; Manager/Provider) — render as @type.module regardless of source.
((import_specifier
  name: (identifier) @type.module)
  (#match? @type.module "(Module|Service|Util|Utils|Helper|Manager|Provider)$"))

((import_specifier
  alias: (identifier) @type.module)
  (#match? @type.module "(Module|Service|Util|Utils|Helper|Manager|Provider)$"))

; Scoped-package PascalCase imports (`@fluentui/...`, `@azure/...`, …) —
; treat as module/component references.
((import_statement
  (import_clause
    (named_imports
      (import_specifier
        name: (identifier) @type.module)))
  source: (string) @_src)
  (#match? @type.module "^[A-Z][a-z]")
  (#match? @_src "^[\"']@[a-z]"))

((import_statement
  (import_clause
    (named_imports
      (import_specifier
        alias: (identifier) @type.module)))
  source: (string) @_src)
  (#match? @type.module "^[A-Z][a-z]")
  (#match? @_src "^[\"']@[a-z]"))

; Literals
(this) @variable.special

(super) @variable.special

[
  (null)
  (undefined)
] @keyword

[
  (true)
  (false)
] @boolean

(literal_type
  [
    (null)
    (undefined)
    (true)
    (false)
  ] @type.builtin)

(comment) @comment

(hash_bang_line) @comment

[
  (string)
  (template_string)
  (template_literal_type)
] @string

(escape_sequence) @string.escape

(regex) @string.regex

(regex_flags) @keyword.operator.regex

(number) @number

; Tokens
[
  ";"
  "?."
  "."
  ","
  ":"
  "?"
] @punctuation.delimiter

[
  "..."
  "-"
  "--"
  "-="
  "+"
  "++"
  "+="
  "*"
  "*="
  "**"
  "**="
  "/"
  "/="
  "%"
  "%="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "==="
  "!"
  "!="
  "!=="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "~"
  "^"
  "&"
  "|"
  "^="
  "&="
  "|="
  "&&"
  "||"
  "??"
  "&&="
  "||="
  "??="
  "..."
] @operator

(regex
  "/" @string.regex)

(ternary_expression
  [
    "?"
    ":"
  ] @operator)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(template_type
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(decorator
  "@" @punctuation.special)

(union_type
  "|" @operator)

(intersection_type
  "&" @operator)

(type_annotation
  ":" @operator)

(index_signature
  ":" @operator)

(type_predicate_annotation
  ":" @operator)

(public_field_definition
  "?" @punctuation.special)

(property_signature
  "?" @punctuation.special)

(method_signature
  "?" @punctuation.special)

(optional_parameter
  ([
    "?"
    ":"
  ]) @punctuation.special)

; Keywords
[
  "abstract"
  "as"
  "async"
  "debugger"
  "declare"
  "default"
  "delete"
  "extends"
  "get"
  "implements"
  "in"
  "infer"
  "instanceof"
  "is"
  "keyof"
  "module"
  "namespace"
  "new"
  "of"
  "override"
  "private"
  "protected"
  "public"
  "readonly"
  "satisfies"
  "set"
  "static"
  "target"
  "typeof"
  "using"
  "void"
  "with"
] @keyword

[
  "const"
  "let"
  "var"
  "function"
  "class"
  "enum"
  "interface"
  "type"
] @keyword.declaration

[
  "export"
  "from"
  "import"
] @keyword.import

[
  "await"
  "break"
  "case"
  "catch"
  "continue"
  "do"
  "else"
  "finally"
  "for"
  "if"
  "return"
  "switch"
  "throw"
  "try"
  "while"
  "yield"
] @keyword.control

(switch_default
  "default" @keyword.control)
