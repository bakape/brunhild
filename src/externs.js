'use strict'

mergeInto(LibraryManager.library, {
	set_outer_html: function (id, html) {
		var el = document.getElementById(Pointer_stringify(id))
		if (el) {
			el.outerHTML = Pointer_stringify(html)
		}
	},
	set_inner_html: function (id, html) {
		var el = document.getElementById(Pointer_stringify(id))
		if (el) {
			el.innerHTML = Pointer_stringify(html)
		}
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
		var el = document.getElementById(Pointer_stringify(id))
		if (!el) {
			return
		}
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		el.appendChild(cont.firstChild)
	},
	prepend: function (id, html) {
		var el = document.getElementById(Pointer_stringify(id))
		if (!el) {
			return
		}
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		el.insertBefore(cont.firstChild, el.firstChild)
	},
	before: function (id, html) {
		var el = document.getElementById(Pointer_stringify(id))
		if (!el) {
			return
		}
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		el.parentNode.insertBefore(cont.firstChild, el)
	},
	after: function (id, html) {
		var el = document.getElementById(Pointer_stringify(id))
		if (!el) {
			return
		}
		var cont = document.createElement('div')
		cont.innerHTML = Pointer_stringify(html)
		el.parentNode.insertBefore(cont.firstChild, el.nextSibling)
	},
	set_attr: function (id, key, val) {
		var el = document.getElementById(Pointer_stringify(id))
		if (!el) {
			return
		}
		el.setAttribute(Pointer_stringify(key), Pointer_stringify(val))
	},
	remove_attr: function (id, key) {
		var el = document.getElementById(Pointer_stringify(id))
		if (!el) {
			return
		}
		el.removeAttribute(Pointer_stringify(key))
	}
})
