import { ReactNode } from "react";
import { Container, Nav, Navbar, Spinner } from "react-bootstrap";

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="layout-root">
      <Navbar bg="light" expand="lg">
        <Nav className="mr-auto">
          <Navbar.Brand>Stracciatella Toolset</Navbar.Brand>
          <Nav.Link>Change Mods</Nav.Link>
        </Nav>
      </Navbar>
      <Container>
          {children}
      </Container>
    </div>
  );
}
