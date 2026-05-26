; Variables
(identifier) @variable

(call_expression
  function: (member_expression
    object: (identifier) @type
    (#any-of? @type
      "Promise" "Array" "Object" "Map" "Set" "WeakMap" "WeakSet" "Date" "Error" "TypeError"
      "RangeError" "SyntaxError" "ReferenceError" "EvalError" "URIError" "RegExp" "Function"
      "Number" "String" "Boolean" "Symbol" "BigInt" "Proxy" "ArrayBuffer" "DataView")))

; Import identifier names — uniform color across default, named, alias, namespace
; imports to match WebStorm JS.MODULE_NAME behavior. Type imports above remain
; cyan via more-specific captures.
(import_clause
  (identifier) @variable.import)

(import_specifier
  name: (identifier) @variable.import)

(import_specifier
  alias: (identifier) @variable.import)

(namespace_import
  (identifier) @variable.import)

; Properties
(property_identifier) @property

(shorthand_property_identifier) @property

(shorthand_property_identifier_pattern) @property

(private_property_identifier) @property

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

; Special identifiers
;
(type_identifier) @type

(predefined_type) @type.builtin

(class_declaration
  (type_identifier) @type.class)

(extends_clause
  value: (identifier) @type.class)

([
  (identifier)
  (shorthand_property_identifier)
  (shorthand_property_identifier_pattern)
] @constant
  (#match? @constant "^_*[A-Z_][A-Z\\d_]*$"))

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

(comment) @comment

(hash_bang_line) @comment

[
  (string)
  (template_string)
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
] @punctuation.delimiter

[
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

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(ternary_expression
  [
    "?"
    ":"
  ] @operator)

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
  "instanceof"
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

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(decorator
  "@" @punctuation.special)

; JSX elements
(jsx_opening_element
  [
    (identifier) @type @tag.component.jsx
    (member_expression
      object: (identifier) @type @tag.component.jsx
      property: (property_identifier) @type @tag.component.jsx)
    (member_expression
      object: (member_expression
        object: (identifier) @type @tag.component.jsx
        property: (property_identifier) @type @tag.component.jsx)
      property: (property_identifier) @type @tag.component.jsx)
    (member_expression
      object: (member_expression
        object: (member_expression
          object: (identifier) @type @tag.component.jsx
          property: (property_identifier) @type @tag.component.jsx)
        property: (property_identifier) @type @tag.component.jsx)
      property: (property_identifier) @type @tag.component.jsx)
  ])

(jsx_closing_element
  [
    (identifier) @type @tag.component.jsx
    (member_expression
      object: (identifier) @type @tag.component.jsx
      property: (property_identifier) @type @tag.component.jsx)
    (member_expression
      object: (member_expression
        object: (identifier) @type @tag.component.jsx
        property: (property_identifier) @type @tag.component.jsx)
      property: (property_identifier) @type @tag.component.jsx)
    (member_expression
      object: (member_expression
        object: (member_expression
          object: (identifier) @type @tag.component.jsx
          property: (property_identifier) @type @tag.component.jsx)
        property: (property_identifier) @type @tag.component.jsx)
      property: (property_identifier) @type @tag.component.jsx)
  ])

(jsx_self_closing_element
  [
    (identifier) @type @tag.component.jsx
    (member_expression
      object: (identifier) @type @tag.component.jsx
      property: (property_identifier) @type @tag.component.jsx)
    (member_expression
      object: (member_expression
        object: (identifier) @type @tag.component.jsx
        property: (property_identifier) @type @tag.component.jsx)
      property: (property_identifier) @type @tag.component.jsx)
    (member_expression
      object: (member_expression
        object: (member_expression
          object: (identifier) @type @tag.component.jsx
          property: (property_identifier) @type @tag.component.jsx)
        property: (property_identifier) @type @tag.component.jsx)
      property: (property_identifier) @type @tag.component.jsx)
  ])

(jsx_opening_element
  (identifier) @tag.jsx
  (#match? @tag.jsx "^[a-z][^.]*$"))

(jsx_closing_element
  (identifier) @tag.jsx
  (#match? @tag.jsx "^[a-z][^.]*$"))

(jsx_self_closing_element
  (identifier) @tag.jsx
  (#match? @tag.jsx "^[a-z][^.]*$"))

(jsx_attribute
  (property_identifier) @attribute.jsx)

(jsx_opening_element
  ([
    "<"
    ">"
  ]) @punctuation.bracket.jsx)

(jsx_closing_element
  ([
    "</"
    ">"
  ]) @punctuation.bracket.jsx)

(jsx_self_closing_element
  ([
    "<"
    "/>"
  ]) @punctuation.bracket.jsx)

(jsx_attribute
  "=" @punctuation.delimiter.jsx)

(jsx_text) @text.jsx

(html_character_reference) @string.special
