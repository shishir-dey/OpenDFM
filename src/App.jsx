import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  CloseIcon,
  FileIcon,
  GithubIcon,
  LayersIcon,
  UploadIcon,
} from './icons.jsx';

const MAX_FILE_SIZE = 50 * 1024 * 1024;
const LOGO_URL = `${import.meta.env.BASE_URL}OpenDFM.png`;
const SUPPORTED_EXTENSIONS = [
  '.zip', '.rar', '.gbr', '.ger', '.gtl', '.gbl', '.gts', '.gbs', '.gto', '.gbo',
  '.gko', '.gm1', '.drl', '.xln', '.txt', '.art', '.pho',
];

const DFM_ENGINES = [
  ['generic', 'Generic'],
  ['jlcpcb', 'JLCPCB'],
  ['nextpcb', 'NextPCB'],
  ['pcbway', 'PCBWay'],
  ['eurocircuits', 'Eurocircuits'],
  ['oshpark', 'OSH Park'],
  ['seeed', 'Seeed Fusion'],
];

const DFM_ANALYSIS = [
  {
    title: 'Routing layer analysis',
    rules: [
      ['Sharp trace corner', '0', 'pass'],
      ['BGA pad', '0', 'pass'],
      ['Via placed within a pad', '0', 'pass'],
      ['Trace to board edge', '0.00 mm', 'pass'],
      ['Trace spacing', '0.00 mm', 'pass'],
      ['Unconnected trace end', '0', 'pass'],
      ['Trace width', '0.00 mm', 'pass'],
      ['Fiducial', '0', 'pass'],
      ['Pad to board edge', '0.00 mm', 'pass'],
      ['Pad spacing', '0.00 mm', 'pass'],
      ['Plated through-hole to trace clearance', '0.00 mm', 'pass'],
      ['Annular ring', '0.00 mm', 'pass'],
      ['THT to SMD', '0.00 mm', 'pass'],
      ['Via to pad', '0.00 mm', 'pass'],
    ],
  },
  {
    title: 'Soldermask layer analysis',
    rules: [
      ['Soldermask bridge', '0.00 mm', 'pass'],
      ['Solder mask opening exposing trace', '0', 'pass'],
      ['Soldermask opening with multiple segments', '0', 'pass'],
      ['Negative soldermask expansion', '0.00 mm', 'pass'],
    ],
  },
  {
    title: 'Silkscreen layer analysis',
    rules: [
      ['Silkscreen to pad', '0.00 mm', 'pass'],
      ['Silkscreen to hole', '0.00 mm', 'pass'],
      ['Silkscreen line width', '0.00 mm', 'pass'],
    ],
  },
  {
    title: 'Drill layer',
    rules: [
      ['Unconnected via', '0', 'pass'],
      ['Missing plated through-hole', '0', 'pass'],
      ['Unconnected via', '0', 'pass'],
      ['Plated through-hole spacing', '0.00 mm', 'pass'],
      ['Short slot detection', '0', 'pass'],
      ['Slot width check', '0.00 mm', 'pass'],
      ['Via to PTH spacing', '0.00 mm', 'pass'],
      ['Unconnected via', '0', 'pass'],
    ],
  },
];

const STACKUP_METRICS = [
  ['PCB layers', '0'],
  ['Board thickness', '0.00 mm'],
  ['PCB size', '0 × 0 mm'],
  ['Minimum track width', '0.00 mm'],
  ['Minimum track clearance', '0.00 mm'],
  ['Minimum hole diameter', '0.00 mm'],
  ['Minimum annular ring', '0.00 mm'],
  ['Hole count', '0'],
  ['Hole density', '0 / cm²'],
  ['Length of milling', '0.00 mm'],
  ['Half holes', '0'],
  ['ENIG area', '0.00 mm²'],
  ['ENIG percentage', '0%'],
  ['Gold fingers', '0'],
  ['Flying probe point count', '0'],
  ['Pad count', '0'],
];

function isSupported(file) {
  const name = file.name.toLowerCase();
  return SUPPORTED_EXTENSIONS.some((extension) => name.endsWith(extension));
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
}

function RuleGroup({ title, rules }) {
  return (
    <section className="analysis-group">
      <div className="analysis-group-heading">
        <h3>{title}</h3>
        <span>{rules.length} checks</span>
      </div>
      <div className="rule-table">
        <div className="rule-table-heading">
          <span>Check</span>
          <span>Detected value</span>
        </div>
        {rules.map(([label, value, tone], index) => (
          <div className="rule-row" key={`${label}-${index}`}>
            <span className="rule-name">{label}</span>
            <span className={`detected-value ${tone}`}>
              <i aria-hidden="true" />
              {value}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}

function App() {
  const inputRef = useRef(null);
  const engineRef = useRef(null);
  const [files, setFiles] = useState([]);
  const [error, setError] = useState('');
  const [dragging, setDragging] = useState(false);
  const [activeTab, setActiveTab] = useState('dfm');
  const [dfmEngine, setDfmEngine] = useState('generic');
  const [engineOpen, setEngineOpen] = useState(false);

  useEffect(() => {
    const closeEngineMenu = (event) => {
      if (event.type === 'keydown' && event.key !== 'Escape') return;
      if (event.type === 'pointerdown' && engineRef.current?.contains(event.target)) return;
      setEngineOpen(false);
    };

    document.addEventListener('pointerdown', closeEngineMenu);
    document.addEventListener('keydown', closeEngineMenu);
    return () => {
      document.removeEventListener('pointerdown', closeEngineMenu);
      document.removeEventListener('keydown', closeEngineMenu);
    };
  }, []);

  const selectedEngine = DFM_ENGINES.find(([value]) => value === dfmEngine)?.[1] ?? 'Generic';

  const addFiles = useCallback((fileList) => {
    const incoming = Array.from(fileList ?? []);
    if (!incoming.length) return;

    const oversized = incoming.find((file) => file.size > MAX_FILE_SIZE);
    if (oversized) {
      setError(`${oversized.name} is over the 50 MB per-file limit.`);
      return;
    }

    const unsupported = incoming.find((file) => !isSupported(file));
    if (unsupported) {
      setError(`${unsupported.name} is not a supported Gerber, drill, or ZIP file.`);
      return;
    }

    setError('');
    setFiles((current) => {
      const byIdentity = new Map(
        current.map((file) => [`${file.name}:${file.size}:${file.lastModified}`, file]),
      );
      incoming.forEach((file) => {
        byIdentity.set(`${file.name}:${file.size}:${file.lastModified}`, file);
      });
      return Array.from(byIdentity.values());
    });
  }, []);

  const totalSize = useMemo(
    () => files.reduce((total, file) => total + file.size, 0),
    [files],
  );

  const removeFile = (fileToRemove) => {
    setFiles((current) => current.filter((file) => file !== fileToRemove));
  };

  const onDrop = (event) => {
    event.preventDefault();
    setDragging(false);
    addFiles(event.dataTransfer.files);
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand" aria-label="OpenDFM">
          <img className="brand-logo" src={LOGO_URL} alt="OpenDFM" />
          <span className="brand-tagline">Gerber manufacturability checks</span>
        </div>
        <div className="header-actions">
          <input
            ref={inputRef}
            className="file-input"
            type="file"
            multiple
            accept={SUPPORTED_EXTENSIONS.join(',')}
            onChange={(event) => {
              addFiles(event.target.files);
              event.target.value = '';
            }}
          />
          <div className="engine-picker" ref={engineRef}>
            <button
              className={`engine-select ${engineOpen ? 'open' : ''}`}
              type="button"
              aria-haspopup="listbox"
              aria-expanded={engineOpen}
              onClick={() => setEngineOpen((open) => !open)}
            >
              <span>DFM Engine</span>
              <strong>{selectedEngine}</strong>
              <i aria-hidden="true" />
            </button>
            {engineOpen && (
              <div className="engine-menu" role="listbox" aria-label="DFM Engine">
                <p>Fabrication profile</p>
                {DFM_ENGINES.map(([value, label]) => (
                  <button
                    className={dfmEngine === value ? 'selected' : ''}
                    type="button"
                    role="option"
                    aria-selected={dfmEngine === value}
                    key={value}
                    onClick={() => {
                      setDfmEngine(value);
                      setEngineOpen(false);
                    }}
                  >
                    <span>{label}</span>
                    <i aria-hidden="true" />
                  </button>
                ))}
              </div>
            )}
          </div>
          <button className="upload-button" type="button" onClick={() => inputRef.current?.click()}>
            <UploadIcon size={17} />
            <span>Add files</span>
          </button>
          <a
            className="github-link"
            href="https://github.com/shishir-dey/OpenDFM"
            target="_blank"
            rel="noreferrer"
            aria-label="View OpenDFM on GitHub"
          >
            <GithubIcon />
          </a>
        </div>
      </header>

      <main>
        <section className="workspace-section">
          <div
            className={`workspace-card ${dragging ? 'dragging' : ''}`}
            onDragEnter={(event) => { event.preventDefault(); setDragging(true); }}
            onDragOver={(event) => event.preventDefault()}
            onDragLeave={(event) => {
              if (!event.currentTarget.contains(event.relatedTarget)) setDragging(false);
            }}
            onDrop={onDrop}
          >
            <div className="project-layout">
              <section className="gerber-pane" aria-label="Gerber preview">
                {files.length === 0 ? (
                  <div className="empty-state">
                    <div className="drop-icon"><LayersIcon size={26} /></div>
                    <h1>Check your Gerber files</h1>
                    <p>Drop a fabrication ZIP or select the individual Gerber and drill files.</p>
                    <button type="button" onClick={() => inputRef.current?.click()}>
                      <UploadIcon size={18} /> Choose files
                    </button>
                  </div>
                ) : (
                  <>
                    <div className="gerber-toolbar">
                      <div>
                        <span>Gerber preview</span>
                        <strong>{files.length} {files.length === 1 ? 'file' : 'files'} · {formatBytes(totalSize)}</strong>
                      </div>
                    </div>
                    <div id="gerber-svg-viewport" className="svg-viewport" />
                    <div className="selected-files-strip" aria-label="Selected fabrication files">
                      {files.map((file) => (
                        <div className="file-chip" key={`${file.name}:${file.size}:${file.lastModified}`}>
                          <span className="file-type"><FileIcon /></span>
                          <span className="file-details">
                            <strong title={file.name}>{file.name}</strong>
                            <small>{formatBytes(file.size)}</small>
                          </span>
                          <button type="button" onClick={() => removeFile(file)} aria-label={`Remove ${file.name}`}>
                            <CloseIcon />
                          </button>
                        </div>
                      ))}
                    </div>
                  </>
                )}
              </section>
                <section className="results-panel">
                  <div className="results-tabs" role="tablist" aria-label="Analysis views">
                    <button
                      className={activeTab === 'dfm' ? 'active' : ''}
                      type="button"
                      role="tab"
                      aria-selected={activeTab === 'dfm'}
                      onClick={() => setActiveTab('dfm')}
                    >
                      PCB DFM
                    </button>
                    <button
                      className={activeTab === 'stackup' ? 'active' : ''}
                      type="button"
                      role="tab"
                      aria-selected={activeTab === 'stackup'}
                      onClick={() => setActiveTab('stackup')}
                    >
                      Stackup
                    </button>
                  </div>

                  <div className="results-content">
                    {activeTab === 'dfm' ? (
                      <div className="dfm-report" role="tabpanel">
                        {DFM_ANALYSIS.map((group) => (
                          <RuleGroup key={group.title} title={group.title} rules={group.rules} />
                        ))}
                      </div>
                    ) : (
                      <div className="stackup-report" role="tabpanel">
                        <section className="stackup-section">
                          <div className="analysis-group-heading">
                            <h3>PCB details</h3>
                            <span>{STACKUP_METRICS.length} values</span>
                          </div>
                          <div className="stackup-metrics">
                            {STACKUP_METRICS.map(([label, value]) => (
                              <div className="metric-row" key={label}>
                                <span>{label}</span>
                                <strong>{value}</strong>
                              </div>
                            ))}
                          </div>
                        </section>
                      </div>
                    )}
                  </div>
                </section>
            </div>

            {error && <div className="error-message" role="alert">{error}</div>}
            {dragging && <div className="drop-overlay">Drop files to add them</div>}
          </div>
        </section>
      </main>
    </div>
  );
}

export default App;
