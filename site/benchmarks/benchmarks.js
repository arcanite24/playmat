// Playmat public AI Benchmarks page: renders benchmarks.json into interactive leaderboards, charts, and comparisons.
(function () {
  'use strict';

  var REPO = 'https://github.com/arcanite24/playmat';
  var HF_BASE = 'https://huggingface.co/nerijs/playmat-ai-checkpoints/tree/main/checkpoints/';

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
    (children || []).forEach(function (c) {
      if (c === null || c === undefined || c === false) return;
      if (typeof c === 'string') e.appendChild(document.createTextNode(c));
      else e.appendChild(c);
    });
    return e;
  }

  function svgEl(tag, attrs, children) {
    var e = document.createElementNS('http://www.w3.org/2000/svg', tag);
    if (attrs) {
      Object.keys(attrs).forEach(function (k) {
        if (k === 'text') e.textContent = attrs[k];
        else e.setAttribute(k, String(attrs[k]));
      });
    }
    (children || []).forEach(function (c) { if (c) e.appendChild(c); });
    return e;
  }

  function num(n) { return Number(n).toLocaleString('en-US'); }

  function day(iso) {
    if (!iso) return '';
    var d = new Date(iso + 'T12:00:00Z');
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric', timeZone: 'UTC' });
  }

  function signed(v, digits) {
    var f = digits ? Number(v).toFixed(digits) : String(Math.round(v));
    return (v > 0 ? '+' : '') + f;
  }

  function formatDuration(s) {
    if (s < 1) return s.toFixed(2) + ' s';
    if (s < 60) return s.toFixed(1) + ' s';
    var m = s / 60;
    return m.toFixed(1) + ' min';
  }

  function statCard(bigHtml, label, sub) {
    return el('li', { 'class': 'bm-stat zone' }, [
      el('span', { 'class': 'bm-stat-big', html: bigHtml }),
      el('span', { 'class': 'bm-stat-label', text: label }),
      sub ? el('span', { 'class': 'bm-stat-sub', text: sub }) : null
    ]);
  }

  function miniBar(pct, tone) {
    var clamped = Math.max(0, Math.min(100, pct));
    return el('div', { 'class': 'bm-mini-track' + (tone ? ' ' + tone : '') }, [
      el('span', { style: 'width:' + clamped.toFixed(1) + '%' })
    ]);
  }

  function ckptLink(name, ckpt) {
    if (!ckpt) return el('span', { 'class': 'bm-model-name', text: name });
    return el('a', {
      'class': 'bm-model-name',
      href: HF_BASE + encodeURIComponent(ckpt),
      target: '_blank',
      rel: 'noopener',
      text: name
    });
  }

  // ---------- Tabs ----------

  function initTabs() {
    var tabs = Array.prototype.slice.call(document.querySelectorAll('#tabs .bm-tab'));
    function activate(tabName, updateHash) {
      var found = false;
      tabs.forEach(function (btn) {
        var active = btn.getAttribute('data-tab') === tabName;
        if (active) found = true;
        btn.setAttribute('aria-selected', active ? 'true' : 'false');
        var panel = $('panel-' + btn.getAttribute('data-tab'));
        if (panel) panel.hidden = !active;
      });
      if (!found && tabs.length) {
        activate(tabs[0].getAttribute('data-tab'), false);
        return;
      }
      if (updateHash && history.replaceState) {
        history.replaceState(null, '', '#' + tabName);
      }
    }
    tabs.forEach(function (btn) {
      btn.addEventListener('click', function () {
        activate(btn.getAttribute('data-tab'), true);
      });
    });
    var hash = (location.hash || '').replace(/^#/, '');
    var validHashes = ['gauntlet', 'adapter-runs', 'lineage', 'tactics', 'engines', 'llms'];
    if (validHashes.indexOf(hash) !== -1) activate(hash, false);
  }

  // ---------- Panel 1b: Same-Board Cross-AI Adapter Runs ----------

  function renderAdapterRuns(d) {
    var runs = d.adapter_runs || [];
    var panel = $('panel-adapter-runs');
    panel.textContent = '';

    panel.appendChild(el('div', { 'class': 'bm-panel-head' }, [
      el('h3', { text: 'Same-Board Cross-AI Runs (Playmat Adapter Harness)' }),
      el('span', { 'class': 'bm-panel-meta', text: runs.length + ' recorded harness run' + (runs.length === 1 ? '' : 's') + ' · playmat-ext-v1 (strict hide=1)' })
    ]));
    panel.appendChild(el('p', {
      'class': 'bm-panel-note',
      text: 'Direct apples-to-apples runs where Playmat models, open-source heuristics, and live LLMs play on the exact same Rust rules engine and locked Archbench suites through rust/rl/benchmark_harness.py.'
    }));

    if (!runs.length) {
      panel.appendChild(el('p', { 'class': 'small', text: 'No cross-AI adapter runs published yet. Run rust/rl/benchmark_harness.py --publish-to-site to add one.' }));
      return;
    }

    runs.slice().reverse().forEach(function (run) {
      var title = run.mode === 'duels'
        ? run.suite + ' Duels (' + run.deals + ' deals/pair · ' + day(run.date) + ')'
        : run.suite + ' Decisions (' + run.puzzles + ' puzzles × ' + run.seeds + ' seed' + (run.seeds === 1 ? '' : 's') + ' · ' + day(run.date) + ')';
      panel.appendChild(el('div', { 'class': 'bm-panel-head' }, [
        el('h4', { style: 'margin:8px 0 0;font:700 1.1rem/1.2 var(--font-print);letter-spacing:0.06em;text-transform:uppercase;color:var(--ink-strong)', text: title }),
        el('span', { 'class': 'bm-panel-meta', text: 'Wall time: ' + run.elapsed_s + ' s' })
      ]));

      var tbody = el('tbody');
      if (run.mode === 'duels') {
        var rows = run.leaderboard || [];
        var minE = Math.min.apply(null, rows.map(function (r) { return r.elo; }).concat([950]));
        var maxE = Math.max.apply(null, rows.map(function (r) { return r.elo; }).concat([1300]));
        rows.forEach(function (r, idx) {
          var tel = r.telemetry || null;
          var isExt = !!tel;
          var avgStr = tel ? (tel.avg_latency_ms >= 1000 ? (tel.avg_latency_ms / 1000).toFixed(2) + ' s' : tel.avg_latency_ms.toFixed(1) + ' ms') : (r.worst_decision_ms != null ? r.worst_decision_ms + ' ms worst' : '—');
          var costStr = tel && tel.cost_usd > 0 ? '$' + tel.cost_usd.toFixed(4) : '$0';
          var badge = isExt
            ? el('span', { 'class': 'bm-chip amber', text: 'Adapter' })
            : el('span', { 'class': 'bm-chip fair', text: 'Native Rust' });
          tbody.appendChild(el('tr', { 'class': isExt ? '' : 'is-ours' }, [
            el('td', { 'class': idx === 0 ? 'bm-rank top1' : idx === 1 ? 'bm-rank top2' : 'bm-rank', text: '#' + (idx + 1) }),
            el('td', {}, [
              el('div', {}, [el('span', { 'class': 'bm-model-name', text: r.name }), badge]),
              el('span', { 'class': 'bm-model-sub', text: r.spec })
            ]),
            el('td', { 'class': 'num bm-bar-cell' }, [
              el('span', { 'class': 'bm-elo-strong', text: r.elo.toFixed(1) }),
              miniBar(((r.elo - minE) / Math.max(maxE - minE, 1)) * 100, isExt ? '' : 'sage')
            ]),
            el('td', { 'class': 'num' }, [
              el('strong', { text: r.win_pct.toFixed(1) + '%' }),
              el('span', { 'class': 'bm-model-sub', text: r.won + '-' + r.drew + '-' + r.lost })
            ]),
            el('td', { 'class': 'num', text: avgStr }),
            el('td', { 'class': 'num', text: costStr })
          ]));
        });
        panel.appendChild(el('div', { 'class': 'bm-table-wrap' }, [
          el('table', { 'class': 'bm-table' }, [
            el('thead', {}, [
              el('tr', {}, [
                el('th', { text: 'Rank' }),
                el('th', { text: 'Entrant & Adapter Spec' }),
                el('th', { 'class': 'num', text: 'BT Elo' }),
                el('th', { 'class': 'num', text: 'Win % / W-D-L' }),
                el('th', { 'class': 'num', text: 'Latency' }),
                el('th', { 'class': 'num', text: 'API Cost' })
              ])
            ]),
            tbody
          ])
        ]));
      } else {
        var standings = run.standings || [];
        var catKeys = standings.length && standings[0].categories ? Object.keys(standings[0].categories) : [];
        standings.forEach(function (r, idx) {
          var tel = r.telemetry || null;
          var isExt = !!tel;
          var avgStr = tel ? (tel.avg_latency_ms >= 1000 ? (tel.avg_latency_ms / 1000).toFixed(2) + ' s' : tel.avg_latency_ms.toFixed(1) + ' ms') : 'native';
          var costStr = tel && tel.cost_usd > 0 ? '$' + tel.cost_usd.toFixed(4) : '$0';
          var badge = isExt
            ? el('span', { 'class': 'bm-chip amber', text: 'Adapter' })
            : el('span', { 'class': 'bm-chip fair', text: 'Native Rust' });
          var catSummary = catKeys.map(function (ck) {
            var cv = r.categories[ck];
            return ck.replace(/_/g, ' ') + ': ' + cv.solved + '/' + cv.total;
          }).join(' · ');
          tbody.appendChild(el('tr', { 'class': isExt ? '' : 'is-ours' }, [
            el('td', { 'class': idx === 0 ? 'bm-rank top1' : idx === 1 ? 'bm-rank top2' : 'bm-rank', text: '#' + (idx + 1) }),
            el('td', {}, [
              el('div', {}, [el('span', { 'class': 'bm-model-name', text: r.name }), badge]),
              el('span', { 'class': 'bm-model-sub', text: r.spec }),
              catSummary ? el('span', { 'class': 'bm-model-sub', text: catSummary }) : null
            ]),
            el('td', { 'class': 'num bm-bar-cell' }, [
              el('span', { 'class': 'bm-elo-strong', text: r.pct.toFixed(1) + '%' }),
              el('span', { 'class': 'bm-model-sub', text: r.solved + '/' + r.total }),
              miniBar(r.pct, isExt ? '' : 'sage')
            ]),
            el('td', { 'class': 'num', text: tel ? num(tel.decisions) : '—' }),
            el('td', { 'class': 'num', text: avgStr }),
            el('td', { 'class': 'num', text: costStr })
          ]));
        });
        panel.appendChild(el('div', { 'class': 'bm-table-wrap' }, [
          el('table', { 'class': 'bm-table' }, [
            el('thead', {}, [
              el('tr', {}, [
                el('th', { text: 'Rank' }),
                el('th', { text: 'Entrant & Category Breakdown' }),
                el('th', { 'class': 'num', text: 'Solved' }),
                el('th', { 'class': 'num', text: 'Decisions' }),
                el('th', { 'class': 'num', text: 'Avg Latency' }),
                el('th', { 'class': 'num', text: 'API Cost' })
              ])
            ]),
            tbody
          ])
        ]));
      }
    });
  }

  // ---------- Panel 1: Playmat Fair Gauntlet ----------

  function renderGauntlet(d) {
    var g = d.combined_gauntlet;
    var panel = $('panel-gauntlet');
    var sortKey = 'avg_elo';

    function draw() {
      panel.textContent = '';
      panel.appendChild(el('div', { 'class': 'bm-panel-head' }, [
        el('h3', { text: g.title }),
        el('span', { 'class': 'bm-panel-meta', text: num(g.games) + ' games · ' + g.failed + ' failed · ' + g.gpu_evals + ' GPU evals · ' + day(g.date) })
      ]));
      panel.appendChild(el('p', { 'class': 'bm-panel-note', text: g.note }));

      var select = el('select', { 'class': 'bm-sort', 'aria-label': 'Sort fair gauntlet leaderboard' }, [
        el('option', { value: 'avg_elo', text: 'Combined Avg Elo' }),
        el('option', { value: 'c2_elo', text: 'constructed@2 Elo' }),
        el('option', { value: 'ood_elo', text: 'Unseen OOD Elo' }),
        el('option', { value: 'combined_pct', text: 'Combined Win %' }),
        el('option', { value: 'delta_p3_base', text: 'Delta vs Phase 3 Base' })
      ]);
      select.value = sortKey;
      select.addEventListener('change', function () { sortKey = select.value; draw(); });

      panel.appendChild(el('div', { 'class': 'bm-controls' }, [
        el('span', { 'class': 'small', text: 'Search config: ' + g.search }),
        el('label', { 'class': 'bm-sort-label' }, ['Sort by ', select])
      ]));

      var rows = g.entries.slice().sort(function (a, b) { return b[sortKey] - a[sortKey]; });
      var minElo = 910, maxElo = 1040;

      var tbody = el('tbody');
      rows.forEach(function (r, idx) {
        var rankClass = idx === 0 ? 'bm-rank top1' : idx === 1 ? 'bm-rank top2' : 'bm-rank';
        var badge = r.fair
          ? el('span', { 'class': 'bm-chip fair', text: 'Fair hide=1' })
          : el('span', { 'class': 'bm-chip omni', text: 'Omniscient' });
        var deltaCls = r.delta_p3_base > 0 ? 'bm-delta-pos' : r.delta_p3_base < 0 ? 'bm-delta-neg' : '';

        tbody.appendChild(el('tr', { 'class': r.fair ? 'is-ours' : '' }, [
          el('td', { 'class': rankClass, text: '#' + (idx + 1) }),
          el('td', {}, [
            el('div', {}, [ckptLink(r.name, r.checkpoint), badge]),
            el('span', { 'class': 'bm-model-sub', text: r.params + ' · ' + r.arch }),
            el('span', { 'class': 'bm-model-sub', text: r.note })
          ]),
          el('td', { 'class': 'num bm-bar-cell' }, [
            el('span', { 'class': 'bm-elo-strong', text: r.avg_elo.toFixed(1) }),
            miniBar(((r.avg_elo - minElo) / (maxElo - minElo)) * 100, idx === 0 ? '' : 'sage')
          ]),
          el('td', { 'class': 'num' }, [
            el('strong', { text: String(r.c2_elo) }),
            el('span', { 'class': 'bm-model-sub', text: r.c2_wdl })
          ]),
          el('td', { 'class': 'num' }, [
            el('strong', { text: String(r.ood_elo) }),
            el('span', { 'class': 'bm-model-sub', text: r.ood_wdl })
          ]),
          el('td', { 'class': 'num' }, [
            el('strong', { text: r.combined_pct.toFixed(1) + '%' }),
            el('span', { 'class': 'bm-model-sub', text: r.combined_wdl })
          ]),
          el('td', { 'class': 'num ' + deltaCls, text: signed(r.delta_p3_base, 1) + ' Elo' })
        ]));
      });

      panel.appendChild(el('div', { 'class': 'bm-table-wrap' }, [
        el('table', { 'class': 'bm-table' }, [
          el('thead', {}, [
            el('tr', {}, [
              el('th', { text: 'Rank' }),
              el('th', { text: 'Model & Architecture' }),
              el('th', { 'class': 'num', text: 'Avg Elo' }),
              el('th', { 'class': 'num', text: 'constructed@2' }),
              el('th', { 'class': 'num', text: 'Unseen OOD' }),
              el('th', { 'class': 'num', text: 'Combined' }),
              el('th', { 'class': 'num', text: 'vs P3 Base' })
            ])
          ]),
          tbody
        ])
      ]));
    }
    draw();
  }

  // ---------- Panel 2: Deep Search & Full Lineage ----------

  function renderLineage(d) {
    var ds = d.deep_showcases;
    var panel = $('panel-lineage');
    var filter = 'all';
    var sortKey = 'elo_vs_host';

    function draw() {
      panel.textContent = '';
      panel.appendChild(el('div', { 'class': 'bm-panel-head' }, [
        el('h3', { text: ds.title }),
        el('span', { 'class': 'bm-panel-meta', text: ds.entries.length + ' benchmarked configurations · constructed@2' })
      ]));
      panel.appendChild(el('p', { 'class': 'bm-panel-note', text: ds.note }));

      var filterDefs = [
        { id: 'all', label: 'All models' },
        { id: 'fair', label: 'Fair (hide=1) only' },
        { id: 'neural', label: 'Neural only' },
        { id: 'heuristic', label: 'Heuristics & anchors' }
      ];
      var filtersEl = el('div', { 'class': 'bm-filters' }, filterDefs.map(function (f) {
        var b = el('button', {
          'class': 'bm-filter',
          'aria-pressed': filter === f.id ? 'true' : 'false',
          text: f.label
        });
        b.addEventListener('click', function () { filter = f.id; draw(); });
        return b;
      }));

      var select = el('select', { 'class': 'bm-sort', 'aria-label': 'Sort lineage leaderboard' }, [
        el('option', { value: 'elo_vs_host', text: 'Delta vs omniscient host@s6' }),
        el('option', { value: 'est_random_scale', text: 'Random=1000 scale' }),
        el('option', { value: 'date', text: 'Newest first' })
      ]);
      select.value = sortKey;
      select.addEventListener('change', function () { sortKey = select.value; draw(); });

      panel.appendChild(el('div', { 'class': 'bm-controls' }, [
        filtersEl,
        el('label', { 'class': 'bm-sort-label' }, ['Sort by ', select])
      ]));

      var rows = ds.entries.filter(function (r) {
        if (filter === 'fair') return r.fair;
        if (filter === 'neural') return ['Phase 4', 'Phase 3', 'Wave C', 'Obs-v2', 'Loop 5', 'Track B'].indexOf(r.generation) !== -1;
        if (filter === 'heuristic') return ['Search v2', 'Heuristic', 'Anchor'].indexOf(r.generation) !== -1;
        return true;
      }).sort(function (a, b) {
        if (sortKey === 'date') return (b.date || '').localeCompare(a.date || '');
        return b[sortKey] - a[sortKey];
      });

      var tbody = el('tbody');
      rows.forEach(function (r, idx) {
        var isTop = idx === 0 && sortKey !== 'date';
        var deltaCls = r.elo_vs_host > 0 ? 'bm-delta-pos' : r.elo_vs_host === 0 ? 'bm-delta-amber' : 'bm-delta-neg';
        var badge = r.fair
          ? el('span', { 'class': 'bm-chip fair', text: 'Fair' })
          : el('span', { 'class': 'bm-chip omni', text: 'Omniscient' });
        var barPct = ((r.est_random_scale - 950) / (1600 - 950)) * 100;

        tbody.appendChild(el('tr', { 'class': r.elo_vs_host > 0 ? 'is-ours' : '' }, [
          el('td', { 'class': isTop ? 'bm-rank top1' : 'bm-rank', text: '#' + (idx + 1) }),
          el('td', {}, [
            el('div', {}, [ckptLink(r.name, r.checkpoint), badge]),
            el('span', { 'class': 'bm-model-sub', text: r.generation + ' · ' + r.params + ' · ' + r.search }),
            el('span', { 'class': 'bm-model-sub', text: r.note })
          ]),
          el('td', { 'class': 'num bm-bar-cell' }, [
            el('span', { 'class': 'bm-elo-strong ' + deltaCls, text: signed(r.elo_vs_host, 0) + ' Elo' }),
            miniBar(barPct, r.elo_vs_host > 0 ? '' : 'muted')
          ]),
          el('td', { 'class': 'num' }, [
            el('strong', { text: '~' + r.est_random_scale }),
            el('span', { 'class': 'bm-model-sub', text: r.est_vs_fair_host > 0 ? '~' + signed(r.est_vs_fair_host, 0) + ' vs fair' : '' })
          ]),
          el('td', { 'class': 'num' }, [
            el('span', { text: r.wdl_vs_host }),
            el('span', { 'class': 'bm-model-sub', text: r.games ? num(r.games) + 'g' : '' })
          ]),
          el('td', { 'class': 'num' }, [
            el('span', { text: r.speed }),
            el('span', { 'class': 'bm-model-sub', text: day(r.date) })
          ])
        ]));
      });

      panel.appendChild(el('div', { 'class': 'bm-table-wrap' }, [
        el('table', { 'class': 'bm-table' }, [
          el('thead', {}, [
            el('tr', {}, [
              el('th', { text: 'Rank' }),
              el('th', { text: 'Model & Search' }),
              el('th', { 'class': 'num', text: 'vs host@s6' }),
              el('th', { 'class': 'num', text: 'random=1000' }),
              el('th', { 'class': 'num', text: 'H2H W-D-L' }),
              el('th', { 'class': 'num', text: 'Latency / Date' })
            ])
          ]),
          tbody
        ])
      ]));
    }
    draw();
  }

  // ---------- Panel 3: Tactics & Pro Decisions ----------

  function renderTactics(d) {
    var t = d.tactics;
    var panel = $('panel-tactics');
    panel.textContent = '';

    panel.appendChild(el('div', { 'class': 'bm-panel-head' }, [
      el('h3', { text: 'Tactical & Pro-Decision Suites' }),
      el('span', { 'class': 'bm-panel-meta', text: '4 seeds per puzzle · tactics@2 (31), tactics@3 (18), tactics@4 (16), pro-decisions@1 (16)' })
    ]));
    panel.appendChild(el('p', { 'class': 'bm-panel-note', text: t.note }));

    function scoreCell(obj, emphasize) {
      if (!obj) return el('td', { 'class': 'num', text: '—' });
      var tone = obj.pct >= 90 ? '' : obj.pct >= 75 ? 'sage' : 'muted';
      return el('td', { 'class': 'num bm-bar-cell' }, [
        el('strong', { 'class': emphasize && obj.pct >= 90 ? 'bm-delta-amber' : '', text: obj.pct.toFixed(0) + '%' }),
        el('span', { 'class': 'bm-model-sub', text: obj.solved + '/' + obj.total }),
        miniBar(obj.pct, tone)
      ]);
    }

    var tbody = el('tbody');
    t.entries.forEach(function (r) {
      tbody.appendChild(el('tr', { 'class': r.checkpoint ? 'is-ours' : '' }, [
        el('td', {}, [
          ckptLink(r.name, r.checkpoint),
          el('span', { 'class': 'bm-model-sub', text: r.highlight })
        ]),
        scoreCell(r.tactics2, false),
        scoreCell(r.tactics3, true),
        scoreCell(r.tactics4_full, true),
        scoreCell(r.tactics4_fair, false),
        scoreCell(r.pro_fair, true)
      ]));
    });

    panel.appendChild(el('div', { 'class': 'bm-table-wrap' }, [
      el('table', { 'class': 'bm-table' }, [
        el('thead', {}, [
          el('tr', {}, [
            el('th', { text: 'Model' }),
            el('th', { 'class': 'num', text: 'tactics@2' }),
            el('th', { 'class': 'num', text: 'tactics@3 (fair)' }),
            el('th', { 'class': 'num', text: 'tactics@4 (full)' }),
            el('th', { 'class': 'num', text: 'tactics@4 (fair)' }),
            el('th', { 'class': 'num', text: 'pro-decisions@1 (fair)' })
          ])
        ]),
        tbody
      ])
    ]));
  }

  // ---------- Panel 4: Other MTG AIs ----------

  function renderEngines(d) {
    var ex = d.external_engines;
    var panel = $('panel-engines');
    var filter = 'all';

    function draw() {
      panel.textContent = '';
      panel.appendChild(el('div', { 'class': 'bm-panel-head' }, [
        el('h3', { text: ex.title }),
        el('span', { 'class': 'bm-panel-meta', text: ex.entries.length + ' engines, RL bots & agent systems surveyed' })
      ]));
      panel.appendChild(el('p', { 'class': 'bm-panel-note', text: ex.note }));

      var filterDefs = [
        { id: 'all', label: 'All (' + ex.entries.length + ')' },
        { id: 'ours', label: 'Playmat' },
        { id: 'external', label: 'External MTG AIs' }
      ];
      panel.appendChild(el('div', { 'class': 'bm-controls' }, [
        el('div', { 'class': 'bm-filters' }, filterDefs.map(function (f) {
          var b = el('button', {
            'class': 'bm-filter',
            'aria-pressed': filter === f.id ? 'true' : 'false',
            text: f.label
          });
          b.addEventListener('click', function () { filter = f.id; draw(); });
          return b;
        }))
      ]));

      var items = ex.entries.filter(function (item) {
        if (filter === 'ours') return item.ours;
        if (filter === 'external') return !item.ours;
        return true;
      });

      var grid = el('div', { 'class': 'bm-engines' });
      items.forEach(function (it) {
        var title = it.url
          ? el('a', { href: it.url, target: '_blank', rel: 'noopener', text: it.name })
          : el('span', { text: it.name });
        var chip = el('span', { 'class': 'bm-chip' + (it.ours ? ' amber' : ''), text: it.category });

        grid.appendChild(el('article', { 'class': 'bm-engine' + (it.ours ? ' is-ours' : '') }, [
          el('div', { 'class': 'bm-engine-top' }, [
            el('h4', {}, [title]),
            chip
          ]),
          el('dl', { 'class': 'bm-engine-meta' }, [
            el('div', {}, [el('dt', { text: 'Engine & Card Pool' }), el('dd', { text: it.engine + ' · ' + it.cards })]),
            el('div', {}, [el('dt', { text: 'Speed & Cost' }), el('dd', { text: it.speed_cost })]),
            el('div', {}, [el('dt', { text: 'Fairness / Info' }), el('dd', { text: it.fairness })]),
            el('div', {}, [el('dt', { text: 'Status' }), el('dd', { text: it.status })])
          ]),
          el('div', { 'class': 'bm-engine-lines' }, [
            el('p', {}, [el('strong', { text: 'Method: ' }), it.method]),
            el('p', {}, [el('strong', { text: 'Strengths: ' }), it.strengths]),
            el('p', {}, [el('strong', { text: 'Limits: ' }), it.limits])
          ])
        ]));
      });
      panel.appendChild(grid);
    }
    draw();
  }

  // ---------- Panel 5: Frontier LLMs (mage-bench) ----------

  function renderLlms(d) {
    var lm = d.llm_leaderboard;
    var panel = $('panel-llms');
    var filter = 'all';
    var sortKey = 'elo';

    function draw() {
      panel.textContent = '';
      panel.appendChild(el('div', { 'class': 'bm-panel-head' }, [
        el('h3', { text: lm.title }),
        el('span', { 'class': 'bm-panel-meta', text: lm.total_games + ' XMage games · ' + lm.total_models + ' models total · snapshot ' + day(lm.generated_at) })
      ]));
      panel.appendChild(el('div', { 'class': 'bm-pool-warn' }, [
        el('strong', { text: 'Separate rating pool: ' }),
        lm.scale_note
      ]));

      var filterDefs = [
        { id: 'all', label: 'Top frontier & open-weight' },
        { id: 'open', label: 'Open-weight only' },
        { id: 'closed', label: 'Proprietary only' }
      ];
      var filtersEl = el('div', { 'class': 'bm-filters' }, filterDefs.map(function (f) {
        var b = el('button', {
          'class': 'bm-filter',
          'aria-pressed': filter === f.id ? 'true' : 'false',
          text: f.label
        });
        b.addEventListener('click', function () { filter = f.id; draw(); });
        return b;
      }));

      var select = el('select', { 'class': 'bm-sort', 'aria-label': 'Sort LLM leaderboard' }, [
        el('option', { value: 'elo', text: 'mage-bench Elo (highest)' }),
        el('option', { value: 'win_rate', text: 'Win Rate (highest)' }),
        el('option', { value: 'blunder', text: 'Blunder Score (lowest)' }),
        el('option', { value: 'cost_usd', text: 'API Cost / Game (lowest)' }),
        el('option', { value: 'think_s', text: 'Thinking Time (fastest)' })
      ]);
      select.value = sortKey;
      select.addEventListener('change', function () { sortKey = select.value; draw(); });

      panel.appendChild(el('div', { 'class': 'bm-controls' }, [
        filtersEl,
        el('label', { 'class': 'bm-sort-label' }, ['Sort by ', select])
      ]));

      var rows = lm.entries.filter(function (r) {
        if (filter === 'open') return r.open_weight;
        if (filter === 'closed') return !r.open_weight;
        return true;
      }).sort(function (a, b) {
        if (sortKey === 'blunder' || sortKey === 'cost_usd' || sortKey === 'think_s') {
          return a[sortKey] - b[sortKey];
        }
        return b[sortKey] - a[sortKey];
      });

      var tbody = el('tbody');
      rows.forEach(function (r, idx) {
        var rankCls = idx === 0 ? 'bm-rank top1' : idx === 1 ? 'bm-rank top2' : 'bm-rank';
        var chips = [];
        if (r.open_weight) chips.push(el('span', { 'class': 'bm-chip open', text: 'Open-weight' }));
        if (r.tag) chips.push(el('span', { 'class': 'bm-chip amber', text: r.tag }));

        tbody.appendChild(el('tr', {}, [
          el('td', { 'class': rankCls, text: '#' + r.rank }),
          el('td', {}, [
            el('div', {}, [el('span', { 'class': 'bm-model-name', text: r.name })].concat(chips)),
            el('span', { 'class': 'bm-model-sub', text: r.provider })
          ]),
          el('td', { 'class': 'num bm-bar-cell' }, [
            el('span', { 'class': 'bm-elo-strong', text: String(r.elo) }),
            miniBar(((r.elo - 1550) / (1760 - 1550)) * 100, r.open_weight ? 'sage' : '')
          ]),
          el('td', { 'class': 'num' }, [
            el('strong', { text: r.win_rate.toFixed(1) + '%' }),
            el('span', { 'class': 'bm-model-sub', text: r.games + ' games' })
          ]),
          el('td', { 'class': 'num', text: r.blunder.toFixed(2) }),
          el('td', { 'class': 'num', text: '$' + r.cost_usd.toFixed(2) }),
          el('td', { 'class': 'num', text: formatDuration(r.think_s) })
        ]));
      });

      panel.appendChild(el('div', { 'class': 'bm-table-wrap' }, [
        el('table', { 'class': 'bm-table' }, [
          el('thead', {}, [
            el('tr', {}, [
              el('th', { text: 'Season 1' }),
              el('th', { text: 'Model & Provider' }),
              el('th', { 'class': 'num', text: 'mage-bench Elo' }),
              el('th', { 'class': 'num', text: 'Win Rate' }),
              el('th', { 'class': 'num', text: 'Blunder' }),
              el('th', { 'class': 'num', text: 'Avg Cost' }),
              el('th', { 'class': 'num', text: 'Think / Game' })
            ])
          ]),
          tbody
        ])
      ]));

      panel.appendChild(el('p', { 'class': 'small' }, [
        'Source: ',
        el('a', { href: lm.source_url, target: '_blank', rel: 'noopener', text: lm.source_name }),
        ' (',
        el('a', { href: lm.repo_url, target: '_blank', rel: 'noopener', text: 'GitHub' }),
        '). ',
        lm.other_benchmarks_note
      ]));
    }
    draw();
  }

  // ---------- Progression SVG Chart ----------

  function renderProgression(d) {
    var pts = d.progression || [];
    var host = $('prog-chart');
    if (!pts.length) return;

    var W = 960, H = 330;
    var padL = 62, padR = 36, padT = 32, padB = 64;
    var plotW = W - padL - padR;
    var plotH = H - padT - padB;
    var minY = 950, maxY = 1640;

    function xPos(i) {
      return padL + (pts.length > 1 ? (i / (pts.length - 1)) * plotW : plotW / 2);
    }
    function yPos(val) {
      return padT + plotH - ((val - minY) / (maxY - minY)) * plotH;
    }

    var svg = svgEl('svg', {
      'class': 'bm-svg',
      viewBox: '0 0 ' + W + ' ' + H,
      role: 'img',
      'aria-label': 'Playmat AI strength progression on constructed@2 from random (1000 Elo) to Phase 4 (~1589 Elo)'
    });

    // Horizontal grid lines
    [1000, 1200, 1400, 1600].forEach(function (v) {
      var y = yPos(v);
      svg.appendChild(svgEl('line', {
        x1: padL, y1: y, x2: W - padR, y2: y,
        stroke: 'rgba(233,228,214,0.12)', 'stroke-width': 1
      }));
      svg.appendChild(svgEl('text', {
        x: padL - 10, y: y + 4,
        fill: 'rgba(233,228,214,0.55)',
        'font-size': 12,
        'text-anchor': 'end',
        'font-family': 'Barlow Condensed, sans-serif',
        text: String(v)
      }));
    });

    // Dashed reference line for omniscient host@s6 (1437)
    var hostY = yPos(1437);
    svg.appendChild(svgEl('line', {
      x1: padL, y1: hostY, x2: W - padR, y2: hostY,
      stroke: 'rgba(143,193,154,0.55)', 'stroke-width': 1.5, 'stroke-dasharray': '5 5'
    }));
    svg.appendChild(svgEl('text', {
      x: padL + 8, y: hostY - 7,
      fill: '#8fc19a',
      'font-size': 12,
      'font-weight': 700,
      'font-family': 'Barlow Condensed, sans-serif',
      'letter-spacing': '0.06em',
      text: 'OMNISCIENT HOST@S6 BASELINE (1437)'
    }));

    // Area fill under curve
    var lineCoords = pts.map(function (p, i) { return xPos(i).toFixed(1) + ',' + yPos(p.random_scale).toFixed(1); });
    var areaPath = 'M ' + xPos(0).toFixed(1) + ',' + (padT + plotH).toFixed(1) +
      ' L ' + lineCoords.join(' L ') +
      ' L ' + xPos(pts.length - 1).toFixed(1) + ',' + (padT + plotH).toFixed(1) + ' Z';
    svg.appendChild(svgEl('path', {
      d: areaPath,
      fill: 'rgba(240,177,61,0.10)'
    }));

    // Main polyline
    svg.appendChild(svgEl('polyline', {
      points: lineCoords.join(' '),
      fill: 'none',
      stroke: '#f0b13d',
      'stroke-width': 3,
      'stroke-linejoin': 'round',
      'stroke-linecap': 'round'
    }));

    // Points and labels
    pts.forEach(function (p, i) {
      var cx = xPos(i);
      var cy = yPos(p.random_scale);
      var isPeak = i === pts.length - 1;
      var aboveHost = p.elo_vs_host > 0;
      svg.appendChild(svgEl('circle', {
        cx: cx, cy: cy,
        r: isPeak ? 6.5 : 4.5,
        fill: aboveHost ? '#f0b13d' : (p.elo_vs_host === 0 ? '#8fc19a' : '#e9e4d6'),
        stroke: '#0b120f',
        'stroke-width': 2
      }));
      // Value above point
      svg.appendChild(svgEl('text', {
        x: cx, y: cy - 12,
        fill: aboveHost ? '#ffc65a' : '#e9e4d6',
        'font-size': isPeak ? 14 : 12,
        'font-weight': 700,
        'text-anchor': 'middle',
        'font-family': 'Barlow Condensed, sans-serif',
        text: p.elo_vs_host > 0 ? '+' + p.elo_vs_host : String(p.random_scale)
      }));
      // X-axis stage label
      svg.appendChild(svgEl('text', {
        x: cx, y: padT + plotH + 22,
        fill: '#f6f2e8',
        'font-size': 13,
        'font-weight': 700,
        'text-anchor': 'middle',
        'font-family': 'Barlow Condensed, sans-serif',
        text: p.stage.toUpperCase()
      }));
      svg.appendChild(svgEl('text', {
        x: cx, y: padT + plotH + 39,
        fill: 'rgba(233,228,214,0.58)',
        'font-size': 11,
        'text-anchor': 'middle',
        'font-family': 'Barlow, sans-serif',
        text: p.label
      }));
    });

    host.appendChild(svg);
    host.appendChild(el('div', { 'class': 'bm-chart-legend' }, [
      el('span', {}, [el('i', { style: 'background:#f0b13d' }), 'Fair neural ISMCTS (+Elo shown vs omniscient host@s6)']),
      el('span', {}, [el('i', { style: 'background:#8fc19a' }), 'Omniscient Rust heuristic anchor (host@s6 = 1437 on random=1000 scale)']),
      el('span', {}, [el('i', { style: 'background:#e9e4d6' }), 'Earlier baselines (random=1000 scale)'])
    ]));
  }

  // ---------- Archetype Stress Breakdown ----------

  function renderStress(d) {
    var st = d.stress_suites;
    var grid = $('stress-grid');
    (st.suites || []).forEach(function (s) {
      var neg = s.elo < 0;
      grid.appendChild(el('div', { 'class': 'bm-stress-card zone' }, [
        el('h3', { text: s.name }),
        el('div', { 'class': 'bm-stress-elo' + (neg ? ' neg' : ''), text: signed(s.elo, 0) + ' Elo' }),
        el('div', { 'class': 'bm-stress-shift', text: s.wdl + ' (' + s.pct.toFixed(1) + '%) · ' + s.decks + ' decks' }),
        miniBar(s.pct, neg ? '' : 'sage'),
        el('p', { 'class': 'bm-stress-note', text: 'Pre-Phase-4: ' + s.pre_wdl + ' (' + signed(s.pre_elo, 0) + ' Elo). ' + s.note })
      ]));
    });
  }

  // ---------- Efficiency & Java Comparison ----------

  function renderEfficiencyAndJava(d) {
    var eff = $('eff-box');
    eff.appendChild(el('h3', { text: 'Time & API cost per game' }));
    eff.appendChild(el('p', { 'class': 'small', text: 'Log-scaled bar of average thinking/CPU time per game, from sub-second local Rust search to multi-minute cloud LLMs.' }));

    var list = el('div', { 'class': 'bm-eff-list' });
    var maxLog = Math.log10(2000);
    (d.efficiency_compare || []).forEach(function (row) {
      var cls = row.group === 'Playmat' ? 'ours' : row.group === 'LLM' ? 'llm' : '';
      var pct = Math.max(4, (Math.log10(Math.max(0.5, row.game_time_s) + 0.5) / maxLog) * 100);
      var costStr = row.cost_usd > 0 ? '$' + row.cost_usd.toFixed(2) + '/game' : '$0';
      list.appendChild(el('div', { 'class': 'bm-eff-row ' + cls }, [
        el('div', { 'class': 'bm-eff-head' }, [
          el('strong', { text: row.name }),
          el('span', { text: formatDuration(row.game_time_s) + ' · ' + costStr })
        ]),
        el('div', { 'class': 'bm-eff-track' }, [el('span', { style: 'width:' + pct.toFixed(1) + '%' })]),
        el('span', { 'class': 'bm-eff-sub', text: row.note })
      ]));
    });
    eff.appendChild(list);

    var j = d.java_comparison;
    var jbox = $('java-box');
    jbox.appendChild(el('h3', { text: 'Direct Java XMage vs Playmat Rust AI' }));
    jbox.appendChild(el('p', { 'class': 'small', text: j.note + ' Measured ' + day(j.date) + '.' }));

    var tbody = el('tbody');
    (j.rows || []).forEach(function (r) {
      var isRust = r.name.indexOf('Playmat') === 0;
      tbody.appendChild(el('tr', { 'class': isRust ? 'is-ours' : '' }, [
        el('td', {}, [
          el('span', { 'class': 'bm-model-name', text: r.name }),
          el('span', { 'class': 'bm-model-sub', text: r.note })
        ]),
        el('td', { 'class': 'num' }, [el('strong', { text: r.won + '-' + r.drew + '-' + r.lost })]),
        el('td', { 'class': 'num', text: r.p99_ms >= 1000 ? (r.p99_ms / 1000).toFixed(1) + ' s' : r.p99_ms + ' ms' }),
        el('td', { 'class': 'num', text: r.cpu_game_s.toFixed(2) + ' s' })
      ]));
    });
    jbox.appendChild(el('div', { 'class': 'bm-table-wrap' }, [
      el('table', { 'class': 'bm-table' }, [
        el('thead', {}, [
          el('tr', {}, [
            el('th', { text: 'AI (48 seeded games vs Random)' }),
            el('th', { 'class': 'num', text: 'W-D-L' }),
            el('th', { 'class': 'num', text: 'p99 Move' }),
            el('th', { 'class': 'num', text: 'CPU / Game' })
          ])
        ]),
        tbody
      ])
    ]));
  }

  // ---------- Archbench Suites ----------

  function renderSuites(d) {
    var suites = (d.counted && d.counted.suites) || [];
    var grid = $('suites-grid');
    suites.forEach(function (s) {
      var count = s.kind === 'duels'
        ? num(s.deals) + ' deals'
        : s.puzzles + ' puzzles';
      var meta = s.kind === 'duels'
        ? s.decks + ' locked decks · ' + s.turn_cap + '-turn cap' + (s.mulligans ? ' · London mulligans' : '')
        : (s.categories && s.categories.length ? s.categories.join(', ') : '4 seeds per puzzle');
      grid.appendChild(el('div', { 'class': 'bm-suite zone' }, [
        el('div', { 'class': 'bm-suite-head' }, [
          el('a', {
            'class': 'bm-suite-id',
            href: REPO + '/tree/master/archbench/' + encodeURIComponent(s.folder),
            text: s.id
          }),
          el('span', { 'class': 'bm-suite-count', text: count })
        ]),
        el('p', { text: s.about }),
        el('span', { 'class': 'bm-suite-meta', text: meta + (s.created ? ' · locked ' + day(s.created) : '') })
      ]));
    });
  }

  // ---------- Top-Level Render ----------

  function render(d) {
    var s = d.summary;
    var c = d.counted || {};

    var stamp = $('stamp');
    stamp.textContent = '';
    stamp.appendChild(document.createTextNode('Last updated ' + day(d.updated)));
    if (d.commit) {
      stamp.appendChild(document.createTextNode(' (commit '));
      stamp.appendChild(el('a', { href: REPO + '/commit/' + d.commit, text: d.commit.slice(0, 7) }));
      stamp.appendChild(document.createTextNode(')'));
    }
    stamp.appendChild(document.createTextNode(' · ' + (c.suites ? c.suites.length : 0) + ' locked Archbench suites · ' + num(s.total_corpus_games) + ' self-play games (' + num(s.total_corpus_decisions) + ' decisions).'));

    var stats = $('stats');
    stats.appendChild(statCard(
      '<em>' + s.peak_elo_vs_host + '</em> Elo',
      'peak fair vs omniscient search',
      '~' + s.peak_elo_vs_fair_host + ' Elo vs fair host@s6-fair on constructed@2 (hide=1)'
    ));
    stats.appendChild(statCard(
      '<em>#1</em> OOD',
      'on 38 strictly unseen decks',
      'Phase 4 2.05M Soup (1022 BT Elo, beat every entrant H2H on constructed-ood@1)'
    ));
    stats.appendChild(statCard(
      '<em>' + s.peak_tactics4_pct + '%</em>',
      'tactics@4 full-info record',
      s.peak_tactics4_score + ' seeds solved; 100% tactics@2, 89% tactics@3, 69% fair pro-decisions@1'
    ));
    stats.appendChild(statCard(
      '<em>$0</em> / game',
      'local Rust & GPU inference',
      'vs ' + s.llm_cost_range + ' and ' + s.llm_think_range + ' thinking/game for frontier LLMs'
    ));

    renderGauntlet(d);
    renderAdapterRuns(d);
    renderLineage(d);
    renderTactics(d);
    renderEngines(d);
    renderLlms(d);
    renderProgression(d);
    renderStress(d);
    renderEfficiencyAndJava(d);
    renderSuites(d);
    initTabs();
  }

  function fail() {
    $('stamp').textContent = 'Could not load benchmarks.json. The raw data is in benchmarks.json next to this page.';
  }

  function start() {
    fetch('benchmarks.json', { cache: 'no-cache' })
      .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
      .then(render)
      .catch(fail);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
