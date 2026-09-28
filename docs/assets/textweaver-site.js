/* Accessibility fixes for the textweaver documentation site (Zensical).
   Loaded from zensical.toml's extra_javascript. The site works without
   it; this only adds names, states, and announcements the theme leaves
   out. Written for the Zensical version pinned in
   tools/site-requirements.txt; check it again when that version changes.

   What it does:
   - Buttons made of labels (the navigation menu and the collapsible
     sections) get the button role, a place in the Tab order, Enter and
     Space, and an expanded or collapsed state.
   - Nested lists in the navigation stop being separate landmarks, so the
     landmark list holds the header, the site navigation, "On this page",
     and the footer.
   - The current page in the navigation is marked aria-current="page".
   - The color theme switch starts with the current choice checked.
   - The search box gets a spoken name, its icon buttons get names, the
     search panel is a named dialog, the result count is announced, and
     focus is visible inside it. */
(function () {
  "use strict";

  var SEARCH_BUTTON_NAMES = {
    "lucide-search": "Search",
    "lucide-list-filter": "Show or hide search filters",
    "lucide-x": "Close search",
    "lucide-arrow-left": "Back"
  };

  function checkboxFor(label) {
    var id = label.getAttribute("for");
    if (!id) return null;
    var box = document.getElementById(id);
    return box && box.type === "checkbox" ? box : null;
  }

  /* Labels that toggle a hidden check box act as buttons. */
  function fixToggleLabels() {
    var labels = document.querySelectorAll(
      "label.md-header__button[for='__drawer'], label.md-nav__link[for]"
    );
    Array.prototype.forEach.call(labels, function (label) {
      var box = checkboxFor(label);
      if (!box) return;
      var isDrawer = box.id === "__drawer";
      // Section titles the theme shows as fixed headings (tabindex="")
      // are not controls; leave them as text.
      if (!isDrawer && label.getAttribute("tabindex") !== "0") return;
      label.setAttribute("role", "button");
      if (isDrawer) label.setAttribute("tabindex", "0");
      // A section with its own page shows the page link and an arrow; the
      // arrow is named after the section.
      if (!label.textContent.trim() && !label.getAttribute("aria-label")) {
        var link = label.parentElement && label.parentElement.querySelector("a");
        var name = link ? link.textContent.trim() : "";
        label.setAttribute("aria-label", name ? name + ", show or hide" : "Show or hide section");
      }
      // The state the reader hears is the state on screen: a section
      // starts open (an "indeterminate" box) with navigation.expand, and
      // the theme animates the change, so look again once it settles.
      var shown = function () {
        if (isDrawer) return box.checked;
        var list = label.parentElement && label.parentElement.querySelector(":scope > nav");
        if (!list) return box.checked || box.indeterminate;
        return list.offsetHeight > 0 && getComputedStyle(list).visibility === "visible";
      };
      var sync = function () {
        label.setAttribute("aria-expanded", shown() ? "true" : "false");
        setTimeout(function () {
          label.setAttribute("aria-expanded", shown() ? "true" : "false");
        }, 400);
      };
      sync();
      box.addEventListener("change", sync);
      label.addEventListener("keydown", function (event) {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          event.stopPropagation();
          box.click();
          sync();
        }
      });
    });
    // The click-away overlay is for the mouse only.
    var overlay = document.querySelector("label.md-overlay");
    if (overlay) overlay.setAttribute("aria-hidden", "true");

    // On a narrow window (or at high zoom) the navigation slides in from
    // the side. While it is closed its links sit off screen, where Tab would
    // still reach them with the focus out of sight, so it is inert until
    // the Navigation button opens it.
    var drawer = document.getElementById("__drawer");
    var drawerButton = document.querySelector("label.md-header__button[for='__drawer']");
    var sidebar = document.querySelector(".md-sidebar--primary");
    if (drawer && drawerButton && sidebar) {
      var syncDrawer = function () {
        var sliding = getComputedStyle(drawerButton).display !== "none";
        sidebar.inert = sliding && !drawer.checked;
      };
      syncDrawer();
      drawer.addEventListener("change", syncDrawer);
      window.addEventListener("resize", syncDrawer);
    }
  }

  /* One landmark for the site navigation, not one per nested list. */
  function fixNavLandmarks() {
    var navs = document.querySelectorAll("nav.md-nav");
    Array.prototype.forEach.call(navs, function (nav) {
      if (nav.parentElement && nav.parentElement.closest("nav")) {
        nav.setAttribute("role", "none");
        nav.removeAttribute("aria-label");
      }
    });
    var primary = document.querySelector("nav.md-nav--primary");
    if (primary) primary.setAttribute("aria-label", "Site");
    var path = document.querySelector("nav.md-path");
    if (path) path.setAttribute("aria-label", "Breadcrumb");
  }

  function markCurrentPage() {
    var links = document.querySelectorAll("a.md-nav__link--active");
    Array.prototype.forEach.call(links, function (link) {
      var href = link.getAttribute("href") || "";
      if (href.charAt(0) !== "#") link.setAttribute("aria-current", "page");
    });
  }

  function checkCurrentPalette() {
    var media = document.body.getAttribute("data-md-color-media");
    var inputs = document.querySelectorAll("input.md-option");
    Array.prototype.forEach.call(inputs, function (input) {
      if (input.getAttribute("data-md-color-media") === media) input.checked = true;
    });
  }

  /* The search box lives in a shadow root the theme creates. */
  var SEARCH_STYLE =
    ":focus-visible{outline:3px solid currentColor!important;outline-offset:2px!important}" +
    ".tw-visually-hidden{position:absolute!important;width:1px!important;height:1px!important;" +
    "margin:-1px!important;padding:0!important;overflow:hidden!important;clip:rect(0 0 0 0)!important;" +
    "white-space:nowrap!important;border:0!important}" +
    "a{text-decoration:underline}";

  function fixSearch(root) {
    if (root.__twFixed) return;
    var input = root.querySelector("input");
    var list = root.querySelector("ol");
    if (!input || !list) {
      // The theme fills the shadow root after attaching it; try again then.
      if (!root.__twWatching) {
        root.__twWatching = true;
        new MutationObserver(function () {
          fixSearch(root);
        }).observe(root, { childList: true, subtree: true });
      }
      return;
    }
    root.__twFixed = true;

    var style = document.createElement("style");
    style.textContent = SEARCH_STYLE;
    root.appendChild(style);

    input.setAttribute("aria-label", "Search the documentation");
    // The arrow keys move a highlight a screen reader cannot hear, so the
    // box is a plain search box; Tab reaches the results as links.
    input.setAttribute("role", "searchbox");

    Array.prototype.forEach.call(root.querySelectorAll("button"), function (button) {
      if (button.getAttribute("aria-label") || button.textContent.trim()) return;
      var svg = button.querySelector("svg");
      var cls = svg ? svg.getAttribute("class") || "" : "";
      var name = "Search option";
      Object.keys(SEARCH_BUTTON_NAMES).forEach(function (key) {
        if (cls.indexOf(key) !== -1) name = SEARCH_BUTTON_NAMES[key];
      });
      button.setAttribute("aria-label", name);
      if (svg) svg.setAttribute("aria-hidden", "true");
    });

    var wrapper = root.firstElementChild;
    if (wrapper) {
      wrapper.setAttribute("role", "dialog");
      wrapper.setAttribute("aria-label", "Search");
    }

    // While the search panel is closed the theme only makes it transparent,
    // so its box, buttons, and old results stay in the Tab order and in the
    // screen reader's view after the footer. Make it inert while closed.
    var host = root.host;
    var panel = input;
    while (panel && panel.parentElement && panel.parentElement !== wrapper) {
      panel = panel.parentElement;
    }
    var isOpen = function () {
      return !!panel && getComputedStyle(panel).pointerEvents !== "none";
    };
    var wasOpen = false;
    var syncOpen = function () {
      var open = isOpen();
      if (open && host.inert) {
        host.inert = false;
        input.focus();
      } else if (!open) {
        host.inert = true;
        // Closing returns focus to the Search button, not to the page top.
        var active = document.activeElement;
        if (wasOpen && (!active || active === document.body || active === host)) {
          var opener = document.querySelector(".md-search__button");
          if (opener) opener.focus();
        }
      }
      wasOpen = open;
    };
    if (host && panel && panel !== input) {
      syncOpen();
      new MutationObserver(syncOpen).observe(panel, { attributes: true, attributeFilter: ["class"] });
      // Lift inert before the theme opens the panel, so it can move focus.
      var lift = function () {
        host.inert = false;
      };
      var button = document.querySelector(".md-search__button");
      if (button) button.addEventListener("click", lift, true);
      document.addEventListener("keydown", function (event) {
        // Only the theme's search shortcuts (a letter or "/", or Ctrl+K
        // or Command+K), never Tab or the arrows, which would move focus
        // into the closed panel.
        var t = event.target;
        var typing = t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));
        var shortcut = event.key && event.key.length === 1 &&
          (!typing || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k"));
        if (!shortcut || !host.inert) return;
        lift();
        setTimeout(syncOpen, 50);
      }, true);
    }

    var status = document.createElement("div");
    status.className = "tw-visually-hidden";
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    root.appendChild(status);

    var timer = null;
    var announce = function () {
      clearTimeout(timer);
      timer = setTimeout(function () {
        var query = input.value.trim();
        if (!query) {
          status.textContent = "";
          return;
        }
        var count = list.querySelectorAll(":scope > li").length;
        status.textContent =
          count === 0
            ? "No results for " + query + "."
            : count + (count === 1 ? " result" : " results") +
              " for " + query + ". Press Tab to reach them, or Enter for the first.";
      }, 500);
    };
    new MutationObserver(announce).observe(list, { childList: true });
    input.addEventListener("input", announce);
  }

  function watchForSearch() {
    var scan = function () {
      Array.prototype.forEach.call(document.body.children, function (el) {
        if (el.shadowRoot) fixSearch(el.shadowRoot);
      });
    };
    scan();
    new MutationObserver(scan).observe(document.body, { childList: true });
    // The element around the search button is not itself a dialog.
    var outer = document.querySelector(".md-search[role='dialog']");
    if (outer) {
      outer.removeAttribute("role");
      outer.removeAttribute("aria-label");
    }
  }

  /* Copy buttons on code blocks: a spoken name, and "Copied" announced. */
  function fixCopyButtons() {
    var status = document.createElement("div");
    status.className = "tw-visually-hidden";
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    document.body.appendChild(status);
    var buttons = document.querySelectorAll("button.md-code__button, button.md-clipboard");
    Array.prototype.forEach.call(buttons, function (button) {
      if (!button.getAttribute("aria-label")) {
        button.setAttribute("aria-label", "Copy this code to the clipboard");
      }
      button.addEventListener("click", function () {
        status.textContent = "";
        setTimeout(function () {
          status.textContent = "Copied to the clipboard.";
        }, 100);
      });
    });
  }

  function init() {
    fixCopyButtons();
    fixToggleLabels();
    fixNavLandmarks();
    markCurrentPage();
    checkCurrentPalette();
    watchForSearch();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
