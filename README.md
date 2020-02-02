# brunhild
experimental compressive Rust virtual DOM library

Brunhild aims to provide a minimalistic fast virtual DOM implementation for use
as is or for building higher level (for example, view-based) libraries and
frameworks.

Brunhild's core principle is reduction of allocations and indirection by
internally converting string values to integers, that reference a value in
either a static lookup table of common HTML strings or dynamically populated
global table. This enables most value comparisons and building of element Node
trees to be done much more cheaply.

Brunhild is mostly referenceless in relation to the DOM. Many virtual DOM
libraries create one to one Node <-> DOM Element mappings on Node construction.
Brunhild only performs this, when a DOM Element mutation is required. This
allows to cheaply patch in large subtree changes as HTML strings, reducing FFI
overhead. This is achieved by setting DOM Element IDs and storing those
efficiently as integers on the Node. As a result brunhild does not support
setting the ID attribute by the library user. Please use classes instead for
such purposes.
