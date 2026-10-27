import React from 'react';
import { IconDashboard, IconSettings } from '../common/Icons';

export type TabId = 'dash' | 'settings';

export interface HeaderTabsProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

export const HeaderTabs: React.FC<HeaderTabsProps> = ({ activeTab, onTabChange }) => {
  return (
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
  );
};
