import { PropsWithChildren, ReactNode, useEffect, useMemo } from "react";
import { Alert } from "react-bootstrap";
import { useFetchMods, useMods } from "../state/mods";

import "./WithOpenMod.css";
import { Open } from "../pages/Open";
import { FullSizeLoader } from "./FullSizeLoader";

export function WithOpenMod({ children }: PropsWithChildren<{}>) {
  const { loading, error, selectedMod, editableMods } = useMods();
  const fetchMods = useFetchMods();

  useEffect(() => {
    if (!editableMods) {
      fetchMods();
    }
  }, [editableMods, fetchMods]);

  let content = useMemo(() => {
    let message: ReactNode = children;
    if (error) {
      message = (
        <div className="with-open-mod">
          <Alert variant="danger">{error.toString()}</Alert>
        </div>
      );
    }
    if (loading) {
      message = (
        <div className="with-open-mod">
          <FullSizeLoader />
        </div>
      );
    }
    if (!selectedMod) {
      message = <Open />;
    }

    return message;
  }, [children, error, loading, selectedMod])

  return <>{content}</>;
}
