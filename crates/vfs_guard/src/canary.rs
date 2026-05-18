use std::path::PathBuf;

/// Détecteur de ransomware via fichiers canari (leurres)
///
/// Stratégie:
/// 1. Insérer des fichiers leurres dans l'arborescence virtuelle
/// 2. Détecter les appels Write() sur ces fichiers
/// 3. Déclencher la stase I/O (piège le thread malveillant)
/// 4. Notifier l'EDR (eBPF/ETW) pour profilage mémoire
///
/// TODO: Implémenter selon spécifications NotebookLM > canary detection
pub struct CanaryDetector {
    canary_paths: Vec<PathBuf>,
}

impl CanaryDetector {
    /// Crée un nouveau détecteur avec des chemins canari
    pub fn new(canary_paths: Vec<PathBuf>) -> Self {
        Self { canary_paths }
    }

    /// Vérifie si un chemin est un fichier canari
    pub fn is_canary(&self, path: &PathBuf) -> bool {
        self.canary_paths.contains(path)
    }

    /// Détecte une tentative d'écriture sur un canari
    ///
    /// Retourne true si le comportement est suspect et la stase doit être déclenchée
    pub fn detect_write_attempt(&self, path: &PathBuf) -> bool {
        self.is_canary(path)
    }

    /// Ajoute un nouveau fichier canari
    pub fn add_canary(&mut self, path: PathBuf) {
        if !self.canary_paths.contains(&path) {
            self.canary_paths.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_detection() {
        let canary = PathBuf::from("/tmp/canary.txt");
        let detector = CanaryDetector::new(vec![canary.clone()]);

        assert!(detector.is_canary(&canary));
        assert!(!detector.is_canary(&PathBuf::from("/tmp/normal.txt")));
    }
}
