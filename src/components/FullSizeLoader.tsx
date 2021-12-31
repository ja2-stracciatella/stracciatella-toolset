import { Spinner } from "react-bootstrap";

import "./FullSizeLoader.css";

export function FullSizeLoader() {
  return (
    <div className="full-size-loader">
      <Spinner animation="border" variant="primary" />
    </div>
  );
}
