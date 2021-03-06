import { PropsWithChildren, ReactNode, useEffect, useMemo } from "react";
import { Alert } from "antd";
import { useFetchMods, useMods } from "../state/mods";

import "./WithOpenMod.css";
import { OpenMod } from "./OpenMod";
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
          <Alert type="error" message={error.toString()} />
        </div>
      );
    }
    if (loading) {
      message = (
        <div>
          <FullSizeLoader />
        </div>
      );
    }
    if (!selectedMod) {
      message = <OpenMod />;
    }

    return message;
  }, [children, error, loading, selectedMod]);

  return <>{content}</>;
}
