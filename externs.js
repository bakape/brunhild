'use strict'

mergeInto(LibraryManager.library, {
	set_outer_html: function (id, html) {
		document.getElementById(Pointer_stringify(id))
			.outerHTML = Pointer_stringify(html)
		return
	},
	set_inner_html: function (id, html) {
		document.getElementById(Pointer_stringify(id))
			.innerHTML = Pointer_stringify(html)
		return
	},
	get_inner_html: function (id) {
		var el = document.getElementById(Pointer_stringify(id))
		var s = el ? el.innerHTML : ""
		var len = s.length + 1
		var buf = Module._malloc(len)
		stringToUTF8(s, buf, len)
		return buf
	},
	append: function (id, html) {
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		document.getElementById(Pointer_stringify(id))
			.appendChild(cont.firstChild)
	},
	prepend: function (id, html) {
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		var el = document.getElementById(Pointer_stringify(id))
		el.insertBefore(cont.firstChild, el.firstChild)
	},
	before: function (id, html) {
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		var el = document.getElementById(Pointer_stringify(id))
		el.parentNode.insertBefore(cont.firstChild, el)
	},
	after: function (id, html) {
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		var el = document.getElementById(Pointer_stringify(id))
		el.parentNode.insertBefore(cont.firstChild, el.nextSibling)
	}
})
