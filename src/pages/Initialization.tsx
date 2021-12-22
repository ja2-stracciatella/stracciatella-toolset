import { useEffect } from "react";
import { Alert, Spinner } from "react-bootstrap";
import { useFetchMods, useMods } from "../state/mods";
import { Navigate } from "react-router-dom"

import './Initialization.css'

export function Initialization() {
    const { loading, mods, error } = useMods();
    const fetchMods = useFetchMods();

    useEffect(() => {
        fetchMods();
    }, [fetchMods]);

    let message = <Spinner animation="border" role="status" />;
    if (error) {
        message = <Alert variant="danger">{error.toString()}</Alert>
    } 
    if (!loading && mods) {
        message = <Navigate replace to="/open" />
    }

    return <div className="initialization-root">
        {message}
    </div>
}