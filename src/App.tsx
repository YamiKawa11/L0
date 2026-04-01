import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { 
  Settings as SettingsIcon, 
  Trash2, 
  Copy, 
  Files, 
  X,
  Minus
} from 'lucide-react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

const appWindow = getCurrentWindow();

interface Session {
  id: number;
  created_at: string;
  ended_at: string;
  content: string;
  char_count: number;
}

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeSession, setActiveSession] = useState<Session | null>(null);

  const fetchSessions = async () => {
    try {
      const res = await invoke<Session[]>('get_sessions');
      setSessions(res);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    fetchSessions();

    const unlisten = listen('session-saved', () => {
      fetchSessions();
    });

    return () => {
      unlisten.then(f => f());
    };
  }, []);

  const handleDeleteActive = async () => {
    if (!activeSession) return;
    await invoke('delete_sessions', { ids: [activeSession.id] });
    setActiveSession(null);
    fetchSessions();
  };

  const handleCopyActive = async () => {
    if (!activeSession) return;
    await writeText(activeSession.content);
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleString('ru-RU', { 
      day: '2-digit', month: '2-digit', year: 'numeric',
      hour: '2-digit', minute: '2-digit'
    });
  };

  return (
    <>
      <div 
        className="titlebar" 
        onMouseDown={(e) => {
          if (e.buttons === 1) appWindow.startDragging();
        }}
      >
        <div className="titlebar-controls" onMouseDown={e => e.stopPropagation()}>
          <div className="titlebar-button" onClick={() => setShowSettings(!showSettings)} title="Настройки">
            <SettingsIcon size={14} />
          </div>
          <div className="titlebar-button" onClick={() => appWindow.minimize()} title="Свернуть">
            <Minus size={14} />
          </div>
          <div className="titlebar-button close" onClick={() => appWindow.hide()} title="Скрыть">
            <X size={14} />
          </div>
        </div>
        
        {showSettings && <SettingsPopup onClose={() => setShowSettings(false)} />}
      </div>

      <div className="app-container">
        <div className="sidebar">
          <div className="session-list">
            {sessions.map(session => (
              <div 
                key={session.id} 
                className={`session-item ${activeSession?.id === session.id ? 'selected' : ''}`}
                onClick={() => setActiveSession(session)}
              >
                <div className="session-date">{formatDate(session.created_at)}</div>
                <div className="session-preview">{session.content.substring(0, 45)}...</div>
              </div>
            ))}
          </div>
        </div>
        
        <div className="main-content">
          {activeSession ? (
            <>
              <div className="content-header">
                <div className="session-info">
                  {formatDate(activeSession.created_at)} — {activeSession.char_count} знаков
                </div>
                <div className="content-actions">
                   <button className="icon-btn" onClick={handleCopyActive} title="Копировать всё">
                      <Copy size={16} />
                   </button>
                   <button className="icon-btn danger" onClick={handleDeleteActive} title="Удалить">
                      <Trash2 size={16} />
                   </button>
                </div>
              </div>
              <div className="content-body">
                {activeSession.content}
              </div>
            </>
          ) : (
            <div className="empty-state">
              <Files size={32} strokeWidth={1} style={{ opacity: 0.1, marginBottom: '8px' }} />
              <p style={{ opacity: 0.2 }}>Выберите сессию</p>
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function SettingsPopup({ onClose }: { onClose: () => void }) {
  const [config, setConfig] = useState({
    session_timeout_seconds: '20',
    min_session_length: '20',
    enable_clipboard_capture: 'true',
    start_with_system: 'false',
  });

  useEffect(() => {
    const load = async () => {
      const timeout = await invoke<string>('get_setting', { key: 'session_timeout_seconds', default: '20' });
      const minLen = await invoke<string>('get_setting', { key: 'min_session_length', default: '20' });
      const clip = await invoke<string>('get_setting', { key: 'enable_clipboard_capture', default: 'true' });
      const autostart = await invoke<string>('get_setting', { key: 'start_with_system', default: 'false' });
      setConfig({
        session_timeout_seconds: timeout,
        min_session_length: minLen,
        enable_clipboard_capture: clip,
        start_with_system: autostart,
      });
    };
    load();
  }, []);

  const save = async (key: string, value: string) => {
    await invoke('save_setting', { key, value });
    setConfig(prev => ({ ...prev, [key]: value }));
  };

  return (
    <div className="settings-popup" onMouseDown={e => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '15px' }}>
          <h3>Настройки</h3>
          <button className="icon-btn" onClick={onClose}><X size={14} /></button>
        </div>
        
        <div className="setting-row">
          <label>Таймаут (сек)</label>
          <input 
            type="number" 
            value={config.session_timeout_seconds} 
            onChange={(e) => save('session_timeout_seconds', e.target.value)}
          />
        </div>
        <div className="setting-row">
          <label>Мин. длина</label>
          <input 
            type="number" 
            value={config.min_session_length} 
            onChange={(e) => save('min_session_length', e.target.value)}
          />
        </div>
        <div className="setting-row" style={{ marginTop: '10px' }}>
          <label>Копировать Ctrl+V</label>
          <label className="switch">
            <input 
              type="checkbox" 
              checked={config.enable_clipboard_capture === 'true'} 
              onChange={(e) => save('enable_clipboard_capture', String(e.target.checked))}
            />
            <span className="slider"></span>
          </label>
        </div>
        <div className="setting-row">
          <label>Автозапуск</label>
          <label className="switch">
            <input 
              type="checkbox" 
              checked={config.start_with_system === 'true'} 
              onChange={(e) => {
                const val = String(e.target.checked);
                save('start_with_system', val);
                if (e.target.checked) invoke('plugin:autostart|enable');
                else invoke('plugin:autostart|disable');
              }}
            />
            <span className="slider"></span>
          </label>
        </div>
    </div>
  );
}

export default App;
