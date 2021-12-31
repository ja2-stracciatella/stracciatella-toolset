import { ReactNode, useMemo } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { useMods } from "../state/mods";
import "./Layout.css";
import { ROUTES } from "../EditorRoutes";
import { Container, Navbar, Nav } from "react-bootstrap";

interface LayoutProps {
  children: ReactNode;
}

function SideMenu() {
  const location = useLocation();
  const navigate = useNavigate();
  const menu = useMemo(() => {
    const selectedRoute = ROUTES.find((v) => v.url === location.pathname);
    return (
      <Nav
        className="flex-column"
        activeKey={selectedRoute ? selectedRoute.id : undefined}
      >
        {ROUTES.map((r) => (
          <Nav.Item key={r.id} onClick={() => navigate(r.url)}>
            <Nav.Link eventKey={r.id}>{r.label}</Nav.Link>
          </Nav.Item>
        ))}
      </Nav>
    );
  }, [navigate, location]);

  return menu;
}

export function Layout({ children }: LayoutProps) {
  const { selectedMod } = useMods();
  const modSuffix = useMemo(
    () => (selectedMod ? ` - ${selectedMod.name}` : ""),
    [selectedMod]
  );
  return (
    <>
      <Navbar variant="dark" bg="dark">
        <Container fluid>
          <Navbar.Brand>Stracciatella Toolset{modSuffix}</Navbar.Brand>
        </Container>
      </Navbar>
      <div className="editor-layout">
        <aside>
          <SideMenu />
        </aside>
        <section>
          <div className="editor-layout-content">{children}</div>
        </section>
      </div>
    </>
  );
}
