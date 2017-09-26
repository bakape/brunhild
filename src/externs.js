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
	remove: function (id) {
		var el = document.getElementById(Pointer_stringify(id))
		if (!el) {
			el.remove()
		}
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
	},
	register_listener: function (typ, selector) {
		if (!window.__bh_handlers) {
			window.__bh_handlers = {}
		}

		var type = Pointer_stringify(typ)
		var sel = Pointer_stringify(selector)
		var delegate_event = Module.cwrap(
			"delegate_event",
			null,
			["string", "string", "string"],
		);
		var handler = window.__bh_handlers[type + ":" + sel] = function (e) {
			var el = e.target
			if (sel && !(el.matches && el.matches(sel))) {
				return
			}
			var attrs = {}
			for (var i = 0; i < el.attributes.length; i++) {
				var attr = el.attributes[i]
				attrs[attr.name] = attr.value
			}
			delegate_event(type, sel, JSON.stringify(attrs))
		}

		document.addEventListener(type, handler, { passive: true })
	},
	unregister_listener: function (typ, selector) {
		var type = Pointer_stringify(typ)
		var key = type + ":" + Pointer_stringify(selector)
		document.removeEventListener(type, window.__bh_handlers[key])
		delete window.__bh_handlers[key]
	}
})
