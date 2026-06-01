import { useLocation, useNavigate } from "react-router-dom";
import { useLanguage, type Language } from "../components/LanguageProvider";

export function useLanguageNavigation() {
	const { language, setLanguage } = useLanguage();
	const navigate = useNavigate();
	const location = useLocation();
	const isDocPage = location.pathname.startsWith("/docs/");

	const selectLanguage = (newLang: Language) => {
		setLanguage(newLang);

		if (isDocPage) {
			const pageId = location.pathname.split("/").pop() ?? "";
			navigate(`/docs/${newLang}/${pageId}`);
		}
	};

	return { language, selectLanguage };
}
