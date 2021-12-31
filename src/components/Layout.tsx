import { ReactNode, useMemo } from "react";
import { Layout as AntdLayout, Menu } from "antd";
import { useLocation, useNavigate } from "react-router-dom";

import { useMods } from "../state/mods";
import "./Layout.css";
import { ROUTES } from "../EditorRoutes";

interface LayoutProps {
  children: ReactNode;
}

function SideMenu() {
  const location = useLocation();
  const navigate = useNavigate();
  const menu = useMemo(() => {
    const selectedRoute = ROUTES.find((v) => v.url === location.pathname);
    return (
      <Menu
        theme="light"
        mode="inline"
        selectedKeys={selectedRoute ? [selectedRoute.id] : []}
        className="editor-layout-menu"
      >
        {ROUTES.map((r) => (
          <Menu.Item key={r.id} onClick={() => navigate(r.url)}>
            {r.label}
          </Menu.Item>
        ))}
      </Menu>
    );
  }, [navigate, location]);

  return menu;
}

export function Layout({ children }: LayoutProps) {
  const { selectedMod } = useMods();
  const modSuffix = useMemo(() => selectedMod ? ` - ${selectedMod.name}` : "", [selectedMod]);
  return (
    <AntdLayout className="editor-layout">
      <AntdLayout.Header className="editor-layout-header">
        <h2>Stracciatella Toolset{modSuffix}</h2>
      </AntdLayout.Header>
      <AntdLayout>
        <AntdLayout.Sider width={300} theme="light">
          <SideMenu />
        </AntdLayout.Sider>
        <AntdLayout.Content>
          <div className="editor-layout-content">{children}</div>
        </AntdLayout.Content>
      </AntdLayout>
    </AntdLayout>
  );
}
