import { useEffect } from "react";
import logoEscuro from "../assets/brand/logo-escuro.svg";

/**
 * SOBRE — manifesto Eureka42. Texto oficial fornecido pelo idealizador;
 * a organização em seções e o tratamento tipográfico são de UI, o conteúdo
 * não é reescrito.
 */
export function AboutDialog({ open, onClose }: { open: boolean; onClose(): void }) {
  useEffect(() => {
    if (!open) return;
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === "Escape") onClose(); };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [open, onClose]);

  if (!open) return null;
  return <div className="settings-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <section className="about-dialog" role="dialog" aria-modal="true" aria-labelledby="about-title">
      <header className="about-hero">
        <img src={logoEscuro} alt="Eureka42" className="about-logo" />
        <div className="about-titles">
          <h1 id="about-title">EUREKA<span className="about-42">42</span></h1>
          <p className="about-sub">UNIVERSAL CLI</p>
          <p className="about-tagline">The Freedom to Create.</p>
        </div>
        <button className="settings-close" onClick={onClose} aria-label="Fechar sobre">×</button>
      </header>

      <div className="about-body">
        <p className="about-lead">Não acreditamos em fronteiras para a inteligência.</p>
        <p>Acreditamos que a tecnologia deve ampliar as possibilidades humanas, não restringi-las. Que um desenvolvedor não deve ser limitado pela ferramenta que utiliza, pelo modelo que escolhe ou pela quantidade de tokens que lhe restam.</p>
        <p>A inteligência artificial está transformando a maneira como o mundo constrói software. Mas, enquanto os modelos evoluem, suas interfaces permanecem fragmentadas. Diferentes terminais, diferentes provedores, diferentes limitações e contextos que nem sempre conseguem acompanhar o desenvolvedor.</p>
        <p className="about-lead">Nós decidimos desafiar essa fragmentação.</p>
        <p>Assim nasceu o <strong>Eureka42 — Universal CLI</strong>.</p>
        <p>Um software livre, de código aberto, criado para reunir diferentes interfaces de linha de comando de inteligência artificial em um único ambiente de trabalho.</p>
        <p>Um lugar onde modelos diferentes podem fazer parte de uma mesma jornada de desenvolvimento. Onde o contexto de um projeto pode ser preservado, transportado e reutilizado, mesmo quando uma sessão termina, um limite de tokens é alcançado ou outra ferramenta precisa assumir o trabalho.</p>

        <blockquote className="about-quote">
          <p>Porque ideias não deveriam expirar junto com uma janela de contexto.</p>
          <p>Porque código não deveria pertencer a uma inteligência artificial.</p>
          <p>Porque o desenvolvedor deve continuar no controle.</p>
        </blockquote>

        <h2>Nossa filosofia</h2>
        <p className="about-lead">Uma interface. Múltiplas inteligências. Liberdade de escolha.</p>
        <p>Não buscamos criar mais uma inteligência artificial para competir com as demais.</p>
        <p>Buscamos construir pontes entre elas.</p>
        <p>Acreditamos na interoperabilidade, na transparência, na continuidade do conhecimento e na independência tecnológica.</p>
        <p>Não queremos que o desenvolvedor se adapte aos limites das ferramentas.</p>
        <p className="about-lead">Queremos que as ferramentas se adaptem ao desenvolvedor.</p>
        <p>O Universal CLI nasce para simplificar o complexo, conectar o que está separado e devolver ao programador algo fundamental: a liberdade de escolher como, quando e com qual inteligência deseja trabalhar.</p>

        <h2>O código é livre. O futuro também deve ser.</h2>
        <p>O código aberto não é apenas uma licença.</p>
        <p>É uma declaração de princípios.</p>
        <p>É reconhecer que as melhores tecnologias podem nascer da colaboração entre pessoas que nunca se encontraram, em países diferentes, falando idiomas diferentes, mas compartilhando a mesma vontade de criar.</p>
        <p>O Eureka42 — Universal CLI pertence a essa filosofia.</p>
        <p>Queremos uma comunidade que questione, contribua, aprimore e transforme o software continuamente.</p>
        <p className="about-lead">Sem monopólio de ideias. Sem dependência artificial. Sem fronteiras para a criatividade.</p>

        <h2>Somos parte de algo maior.</h2>
        <p>O Universal CLI é uma das primeiras expressões de uma ambição muito maior: o <strong>Eureka42.org</strong>.</p>
        <p>Nossa iniciativa busca construir o maior framework aberto e colaborativo de inteligência artificial do mundo.</p>
        <p>Um ecossistema internacional de conhecimento, ferramentas, engenharia, pesquisa e desenvolvimento, no qual pessoas e inteligências artificiais possam trabalhar em conjunto para resolver problemas reais.</p>
        <p>Não pretendemos apenas acompanhar o futuro da inteligência artificial.</p>
        <p className="about-lead">Queremos participar de sua construção.</p>

        <h2>O futuro não pertence a um modelo.</h2>
        <p>Não pertence a uma empresa. Não pertence a uma linguagem de programação. Não pertence a uma única maneira de pensar.</p>
        <p>O futuro pertence àqueles que criam, experimentam, compartilham e se recusam a aceitar que os limites de hoje sejam permanentes.</p>

        <blockquote className="about-quote">
          <p>A inteligência não deveria ter fronteiras.</p>
          <p>O conhecimento não deveria ficar aprisionado.</p>
          <p>E a próxima grande ideia não deveria morrer porque acabaram os tokens.</p>
        </blockquote>
      </div>

      <footer className="about-credits">
        <div className="about-credits-brand">
          <strong>EUREKA<span className="about-42">42</span> — UNIVERSAL CLI</strong>
          <span>One Terminal. Infinite Intelligence.</span>
          <em>Built for developers. Powered by freedom.</em>
        </div>
        <div className="about-credits-people">
          <p><span>Idealizador e Programador</span>Raul el Greco</p>
          <p><span>Co-criador</span>Giuseppe Favato</p>
          <p className="about-org"><span>Uma iniciativa</span>Eureka42.org · Open Source Project · 2026</p>
        </div>
      </footer>
    </section>
  </div>;
}
