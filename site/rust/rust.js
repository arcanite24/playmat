// The Rust tracker: draws progress.json (written by rust/tools/progress.py) into the page.
(function () {
  'use strict';

  var REPO = 'https://github.com/arcanite24/playmat';
  var STATUS = { done: 'Done', live: 'Live', active: 'Now', next: 'Next', later: 'Later', parked: 'Parked' };

  function $(id) { return document.getElementById(id); }

  function el(tag, attrs, children) {
    var e = document.createElement(tag);
    if (attrs) {
      Object.keys(attrs).forEach(function (k) {
        if (k === 'text') e.textContent = attrs[k];
        else if (k === 'html') e.innerHTML = attrs[k];
        else if (k === 'style') e.setAttribute('style', attrs[k]);
        else e.setAttribute(k, attrs[k]);
      });
    }
    (children || []).forEach(function (c) { if (c) e.appendChild(c); });
    return e;
  }

  function num(n) { return Number(n).toLocaleString('en-US'); }

  function pct(a, b) { return b ? Math.max(0, Math.min(100, (a / b) * 100)) : 0; }

  function time(v, unit) {
    // milliseconds and microseconds shown the way people read them
    if (unit === 'ms' && v >= 1000) return (v / 1000).toLocaleString('en-US', { maximumFractionDigits: 1 }) + ' s';
    return v.toLocaleString('en-US', { maximumFractionDigits: 1 }) + ' ' + unit;
  }

  function day(iso) {
    if (!iso) return '';
    var d = new Date(iso + 'T12:00:00Z');
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric', timeZone: 'UTC' });
  }

  function chip(s) { return el('span', { 'class': 'rp-chip', 'data-s': s, text: STATUS[s] || s }); }

  function prLinks(prs) {
    if (!prs || !prs.length) return null;
    var p = el('p', { 'class': 'rp-step-meta' });
    p.appendChild(document.createTextNode(prs.length > 1 ? 'Pull requests ' : 'Pull request '));
    prs.forEach(function (n, i) {
      if (i) p.appendChild(document.createTextNode(', '));
      p.appendChild(el('a', { href: REPO + '/pull/' + n, text: '#' + n }));
    });
    return p;
  }

  function track(list, items, gate) {
    items.forEach(function (it) {
      var body = el('div', { 'class': 'rp-step-body' }, [
        el('div', { 'class': 'rp-step-title' }, [el('strong', { text: it.name }), chip(it.status)]),
        el('p', { text: it.text }),
        gate && it.gate ? el('p', { 'class': 'rp-step-meta', text: 'Done when: ' + it.gate }) : null,
        prLinks(it.pr)
      ]);
      list.appendChild(el('li', { 'class': 'rp-step', 'data-s': it.status }, [
        el('span', { 'class': 'rp-dot', 'aria-hidden': 'true', text: it.n }), body
      ]));
    });
  }

  function stat(big, label, sub) {
    return el('li', { 'class': 'rp-stat zone' }, [
      el('span', { 'class': 'rp-stat-big', html: big }),
      el('span', { 'class': 'rp-stat-label', text: label }),
      sub ? el('span', { 'class': 'rp-stat-sub', text: sub }) : null
    ]);
  }

  function times(a, b) {
    var x = a / b;
    return (x >= 10 ? Math.round(x) : x.toFixed(1)).toLocaleString('en-US') + '×';
  }

  function render(d) {
    var c = d.counted;
    var m = {};
    (d.measures || []).forEach(function (x) { m[x.id] = x; });

    $('goal').textContent = d.goal ? 'The goal: ' + d.goal : '';
    var stamp = $('stamp');
    stamp.textContent = '';
    stamp.appendChild(document.createTextNode('Counts as of ' + day(d.generated)));
    if (d.commit) {
      stamp.appendChild(document.createTextNode(' (commit '));
      stamp.appendChild(el('a', { href: REPO + '/commit/' + d.commit, text: d.commit.slice(0, 7) }));
      stamp.appendChild(document.createTextNode(')'));
    }
    stamp.appendChild(document.createTextNode('. Plan last reviewed ' + day(d.updated) + '.'));

    // headline numbers
    var stats = $('stats');
    if (m.node) {
      stats.appendChild(stat('<em>' + times(m.node.java, m.node.rust) + '</em>', 'less CPU per AI search step',
        time(m.node.java, m.node.unit) + ' in Java, ' + time(m.node.rust, m.node.unit) + ' in Rust'));
    }
    if (m.worst) {
      stats.appendChild(stat(time(m.worst.rust, m.worst.unit), 'slowest AI decision',
        'down from ' + time(m.worst.java, m.worst.unit) + ' in Java'));
    }
    var sh = c.shadow;
    stats.appendChild(stat(num(sh.matched) + '<em>/</em>' + num(sh.played), 'shadow games match Java',
      num(sh.checked_in) + ' of them replayed on every change'));
    stats.appendChild(stat(num(c.engine_cards), 'cards in the Rust engine',
      'all ' + num(c.school.cards_in) + ' cards of the ' + c.school.decks + ' school decks'));

    // tracks
    track($('phases'), d.phases || [], false);
    track($('steps'), d.steps || [], true);

    // speed
    var bench = $('bench');
    ['node', 'p99', 'worst'].forEach(function (id) {
      var x = m[id];
      if (!x) return;
      var max = Math.max(x.java, x.rust);
      bench.appendChild(el('div', { 'class': 'rp-vs zone' }, [
        el('h3', { text: x.label }),
        el('div', { 'class': 'rp-vs-row' }, [
          el('span', { 'class': 'rp-vs-who', text: 'Java' }),
          el('span', { 'class': 'rp-vs-track' }, [el('span', { style: 'width:' + pct(x.java, max) + '%' })]),
          el('span', { 'class': 'rp-vs-num', text: time(x.java, x.unit) })
        ]),
        el('div', { 'class': 'rp-vs-row rust' }, [
          el('span', { 'class': 'rp-vs-who', text: 'Rust' }),
          el('span', { 'class': 'rp-vs-track' }, [el('span', { style: 'width:' + pct(x.rust, max) + '%' })]),
          el('span', { 'class': 'rp-vs-num', text: time(x.rust, x.unit) })
        ]),
        el('p', { 'class': 'rp-vs-x', html: times(x.java, x.rust) + '<small>faster</small>' })
      ]));
    });
    if (m.node) {
      var note = m.node.note + ' Measured ' + day(m.node.date) + '.';
      if (m.replay) note += ' For scale: the Rust engine replays a whole game in about ' + time(m.replay.rust, m.replay.unit) + '.';
      bench.appendChild(el('p', { 'class': 'rp-bench-note', text: note }));
    }

    // cost
    var cost = d.cost;
    if (cost) {
      $('cost-note').textContent = cost.note + ' Measured ' + day(cost.date) + '.';
      var box = $('cost');
      var hi = cost.java_ai_game[1];
      var rustTotal = cost.rust_ai_game + cost.rust_copy;
      function row(label, sub, bars, value) {
        return el('div', { 'class': 'rp-cost-row' }, [
          el('div', { 'class': 'rp-cost-label', html: '<strong>' + label + '</strong>' + sub }),
          el('div', { 'class': 'rp-cost-track' }, bars),
          el('div', { 'class': 'rp-cost-num', text: value })
        ]);
      }
      box.appendChild(row('Java AI', 'XMage\'s own AI, real decks', [
        el('span', { 'class': 'rp-cost-range', style: 'width:' + pct(hi, hi) + '%' }),
        el('span', { 'class': 'rp-cost-solid', style: 'width:' + pct(cost.java_ai_game[0], hi) + '%' })
      ], cost.java_ai_game[0] + '–' + cost.java_ai_game[1] + ' s'));
      box.appendChild(row('Rust AI, Java hosting', 'how a game runs today when its decks qualify', [
        el('span', { 'class': 'rp-cost-solid', style: 'width:' + pct(cost.rust_ai_game, hi) + '%' }),
        el('span', { 'class': 'rp-cost-rust', style: 'left:' + pct(cost.rust_ai_game, hi) + '%;width:' + pct(cost.rust_copy, hi) + '%' })
      ], rustTotal.toFixed(2) + ' s'));
      var colors = ['#f0b13d', '#c98f2e', '#8fc19a', '#5f8f78', '#3a5a51', '#e9e4d6'];
      var splitBar = el('div', { 'class': 'rp-split-bar', role: 'img',
        'aria-label': cost.split.map(function (s) { return s.label + ' ' + s.share + '%'; }).join(', ') });
      var keys = el('ul', { 'class': 'rp-split-keys' });
      cost.split.forEach(function (s, i) {
        splitBar.appendChild(el('span', { style: 'flex:' + s.share + ' 1 0;background:' + colors[i % colors.length], title: s.label + ': ' + s.share + '%' }));
        keys.appendChild(el('li', {}, [
          el('i', { style: 'background:' + colors[i % colors.length] }),
          document.createTextNode(s.label + ' '),
          el('b', { text: s.share + '%' })
        ]));
      });
      box.appendChild(el('div', { 'class': 'rp-split' }, [
        el('h3', { text: 'Where Java\'s ' + cost.rust_ai_game + ' s goes' }),
        splitBar, keys,
        el('p', { text: 'The Rust copy, AI included, adds ' + cost.rust_copy + ' s. Building and sending views moves to Rust in step C, and the rest of the game in step D.' })
      ]));
    }

    // cards
    var cov = $('coverage');
    function coverageBar(label, sub, a, b, unit) {
      var attrs = { 'class': 'rp-bar zone' };
      if (a === b) attrs['data-full'] = '';
      return el('div', attrs, [
        el('div', { 'class': 'rp-bar-label', html: '<strong>' + label + '</strong><span>' + sub + '</span>' }),
        el('div', { 'class': 'rp-bar-track', role: 'img', 'aria-label': a + ' of ' + b + ' ' + unit }, [
          el('span', { style: 'width:' + pct(a, b) + '%' })
        ]),
        el('div', { 'class': 'rp-bar-num', html: num(a) + ' <small>of ' + num(b) + '</small>' })
      ]);
    }
    cov.appendChild(coverageBar('School decks', 'The Career\'s Pioneer and Standard decks, ' + num(c.school.cards) + ' different cards',
      c.school.covered, c.school.decks, 'decks'));
    cov.appendChild(coverageBar('Meta decks', 'The 24 Pioneer and Standard decks the AI picks from; ' +
      num(c.meta.cards_in) + ' of their ' + num(c.meta.cards) + ' cards are in', c.meta.covered, c.meta.decks, 'decks'));
    cov.appendChild(coverageBar('Every XMage card', 'Not the target yet: Pioneer first, other formats later',
      c.engine_cards, c.java_card_files, 'cards'));
    var list = $('meta-list');
    c.meta.list.forEach(function (x) {
      var attrs = {};
      if (!x.missing) attrs['data-full'] = '';
      list.appendChild(el('li', attrs, [
        el('span', { text: x.deck.replace(/^(pioneer|standard)-/, function (_, f) { return f.charAt(0).toUpperCase() + f.slice(1) + ' · '; }).replace(/-/g, ' ') }),
        el('b', { text: x.missing ? x.missing + ' cards to go' : 'all in' })
      ]));
    });

    // shadow games
    var sum = $('shadow-sum');
    function fig(big, sub) { return el('div', {}, [el('strong', { text: big }), el('span', { text: sub })]); }
    sum.appendChild(fig(num(sh.played), 'shadow games finished by Java and replayed in Rust'));
    sum.appendChild(fig(num(sh.matched), 'reached the same state at every step'));
    sum.appendChild(fig(String(c.tiers.length), 'tiers of decks'));
    sum.appendChild(fig(num(c.rust.lines), 'lines of Rust across ' + c.rust.crates.length + ' crates'));
    var tiers = $('tiers');
    c.tiers.forEach(function (t) {
      var short = t.what.split(':')[0];
      var rest = t.what.slice(short.length + 1).trim();
      var full = t.played != null && t.matched === t.played;
      var details = el('details', {}, [
        el('summary', {}, [
          el('span', { 'class': 'rp-tier-n', html: '<small>TIER</small>' + t.tier }),
          el('span', { 'class': 'rp-tier-short', text: short }),
          el('span', { 'class': 'rp-tier-score' + (full ? '' : ' partial'), text: t.played != null ? t.matched + '/' + t.played : '' })
        ]),
        el('p', { html: '' }, [])
      ]);
      var p = details.querySelector('p');
      p.textContent = rest ? 'New here: ' + rest + '.' : short;
      var small = el('small', { text: t.note + (t.timeouts ? ' (a game Java didn\'t finish in time has nothing to compare)' : '') });
      p.appendChild(small);
      tiers.appendChild(el('li', { 'class': 'rp-tier' }, [details]));
    });
  }

  function fail() {
    $('stamp').textContent = 'The numbers didn\'t load. They are in progress.json next to this page.';
  }

  function start() {
    fetch('progress.json', { cache: 'no-cache' })
      .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
      .then(render)
      .catch(fail);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
