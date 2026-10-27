import React from 'react';
import { IconDashboard, IconSettings } from '../common/Icons';

export type TabId = 'dash' | 'settings';

export interface HeaderTabsProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

export const HeaderTabs: React.FC<HeaderTabsProps> = ({ activeTab, onTabChange }) => {
  return (
    <header className="gc-header-bar">
      <div className="gc-brand">
        <img
          src="/logo-GC.jpeg"
          alt="GO COMPRESSOR"
          className="gc-brand-logo"
        />
        <div className="gc-brand-title-wrap">
          <span className="gc-brand-title">GO COMPRESSOR</span>
          <span className="gc-brand-version">v0.1.0</span>
        </div>
      </div>

      <div className="gc-tabs" role="tablist" aria-label="Navigation principale">
        <button
          type="button"
          className="gc-tab"
          role="tab"
          id="tab-dash"
          aria-controls="panel-dash"
          aria-selected={activeTab === 'dash'}
          onClick={() => onTabChange('dash')}
        >
          <IconDashboard />
          <span>Tableau de bord</span>
        </button>

        <button
          type="button"
          className="gc-tab"
          role="tab"
          id="tab-settings"
          aria-controls="panel-settings"
          aria-selected={activeTab === 'settings'}
          onClick={() => onTabChange('settings')}
        >
          <IconSettings />
          <span>Paramètres</span>
        </button>
      </div>
    </header>
  );
};
