(function () {
  "use strict";
  var a = document.getElementById("tw-audio");
  var T;
  try { T = JSON.parse(document.getElementById("tw-timeline").textContent); } catch (e) { return; }
  if (!a || !a.play) { return; }
  var S = T.s || [], W = T.w || [];
  var cs = -1, cw = -1, follow = true, raf = 0;
  var mq = window.matchMedia ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
  var main = document.getElementById("text");
  var play = document.getElementById("tw-play");
  var followBtn = document.getElementById("tw-follow");
  // The last entry starting at or before t, if t is before its end.
  function find(L, t) {
    var lo = 0, hi = L.length - 1, r = -1;
    while (lo <= hi) {
      var m = (lo + hi) >> 1;
      if (L[m][0] <= t) { r = m; lo = m + 1; } else { hi = m - 1; }
    }
    return r >= 0 && t < L[r][1] ? r : -1;
  }
  // The sentence playing at t, or the last one started (for back/forward).
  function at(t) {
    var lo = 0, hi = S.length - 1, r = -1;
    while (lo <= hi) {
      var m = (lo + hi) >> 1;
      if (S[m][0] <= t) { r = m; lo = m + 1; } else { hi = m - 1; }
    }
    return r;
  }
  function parts(attr, n) { return main.querySelectorAll("[" + attr + "=\"" + n + "\"]"); }
  function mark(list, cls, on) { for (var i = 0; i < list.length; i++) { list[i].classList.toggle(cls, on); } }
  function inView(el) {
    var r = el.getBoundingClientRect(), h = window.innerHeight || document.documentElement.clientHeight;
    return r.top >= h * 0.2 && r.bottom <= h * 0.9;
  }
  function show(n) {
    var el = document.getElementById("s" + n);
    if (!el || inView(el)) { return; }
    var smooth = !(mq && mq.matches);
    el.scrollIntoView({ block: "center", behavior: smooth ? "smooth" : "auto" });
  }
  function update() {
    var t = a.currentTime * 1000, s = find(S, t), w = find(W, t);
    if (s !== cs) {
      if (cs >= 0) { mark(parts("data-s", cs), "tw-now", false); }
      if (s >= 0) { mark(parts("data-s", s), "tw-now", true); if (follow && !a.paused) { show(s); } }
      cs = s;
    }
    if (w !== cw) {
      if (cw >= 0) { mark(parts("data-w", cw), "tw-word", false); }
      if (w >= 0) { mark(parts("data-w", w), "tw-word", true); }
      cw = w;
    }
  }
  function loop() { update(); raf = a.paused ? 0 : window.requestAnimationFrame(loop); }
  function label() { play.textContent = a.paused ? play.getAttribute("data-play") : play.getAttribute("data-pause"); }
  function seek(ms) { a.currentTime = Math.max(0, ms) / 1000; update(); }
  a.addEventListener("play", function () { label(); if (!raf) { raf = window.requestAnimationFrame(loop); } });
  a.addEventListener("pause", function () { label(); update(); });
  a.addEventListener("ended", function () { label(); update(); });
  a.addEventListener("seeked", update);
  play.addEventListener("click", function () { if (a.paused) { a.play(); } else { a.pause(); } });
  document.getElementById("tw-back").addEventListener("click", function () {
    var t = a.currentTime * 1000, n = at(t);
    // Within the first second of a sentence, back goes to the one before.
    if (n > 0 && t - S[n][0] < 1000) { n -= 1; }
    if (n >= 0) { seek(S[n][0]); }
  });
  document.getElementById("tw-forward").addEventListener("click", function () {
    var n = at(a.currentTime * 1000) + 1;
    if (n < S.length) { seek(S[n][0]); }
  });
  followBtn.addEventListener("click", function () {
    follow = !follow;
    followBtn.setAttribute("aria-pressed", follow ? "true" : "false");
  });
  document.getElementById("tw-speed").addEventListener("change", function () {
    var v = parseFloat(this.value);
    if (v >= 0.5 && v <= 4) { a.playbackRate = v; }
  });
  var chapters = document.querySelectorAll("#tw-controls [data-at]");
  for (var i = 0; i < chapters.length; i++) {
    chapters[i].addEventListener("click", function () { seek(+this.getAttribute("data-at")); a.play(); });
  }
  // Pointer only: a click on a sentence starts there, unless it is on a
  // link or text is being selected.
  main.addEventListener("click", function (e) {
    var sel = window.getSelection ? String(window.getSelection()) : "";
    if (sel || !e.target.closest || e.target.closest("a")) { return; }
    var el = e.target.closest("[data-s]");
    if (el) { var n = +el.getAttribute("data-s"); if (S[n]) { seek(S[n][0]); } }
  });
  document.getElementById("tw-controls").hidden = false;
  label();
})();
