use super::*;

impl WorkspaceApp {
    pub(super) fn render_ftp_form_branch(
        &self,
        active_tab: ConnectionFormTab,
        shows_icon_field: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(form) = self.connection_form_state(cx).form.as_ref() else {
            return div().into_any_element();
        };
        let (
            name,
            host,
            port,
            username,
            group,
            notes,
            path,
            timeout,
            tls,
            save_password,
            icon,
            color,
            bg_color,
            icon_picker_expanded,
        ) = (
            form.name.clone(),
            form.host.clone(),
            form.port.clone(),
            form.username.clone(),
            form.group.clone(),
            form.notes.clone(),
            form.sftp_initial_remote_path.clone(),
            form.connect_timeout_seconds_text.clone(),
            form.ftp_tls,
            form.save_password,
            form.icon.clone(),
            form.color.clone(),
            form.icon_background_color.clone(),
            form.icon_picker_expanded,
        );

        match active_tab {
            ConnectionFormTab::General => {
                let basic = div()
                    .flex()
                    .flex_col()
                    .gap(px(self.tokens.metrics.modal_section_gap))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(self.tokens.metrics.form_host_port_gap))
                            .child(div().flex_1().child(self.render_connection_field(
                                self.i18n.t("modals.new_connection.name"),
                                &name,
                                connection_name_placeholder(
                                    &host,
                                    self.i18n.t("modals.new_connection.name_placeholder"),
                                ),
                                NewConnectionField::Name,
                                false,
                                cx,
                            )))
                            .child(div().flex_1().child(self.render_connection_group_select(
                                self.i18n.t("ssh.form.group"),
                                &group,
                                cx,
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(self.tokens.metrics.form_host_port_gap))
                            .child(div().flex_1().child(self.render_connection_field(
                                self.i18n.t("ssh.form.host"),
                                &host,
                                self.i18n.t("ssh.form.host_placeholder"),
                                NewConnectionField::Host,
                                false,
                                cx,
                            )))
                            .child(div().w(px(self.tokens.metrics.form_port_width)).child(
                                self.render_connection_field(
                                    self.i18n.t("ssh.form.port"),
                                    &port,
                                    "21".into(),
                                    NewConnectionField::Port,
                                    false,
                                    cx,
                                ),
                            )),
                    );

                let authentication = div()
                    .pt(px(self.tokens.spacing.two))
                    .border_t_1()
                    .border_color(rgb(self.tokens.ui.border))
                    .flex()
                    .flex_col()
                    .gap(px(self.tokens.metrics.modal_section_gap))
                    .child(
                        div()
                            .text_size(px(self.tokens.metrics.ui_text_sm))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(self.tokens.ui.text_heading))
                            .child(self.i18n.t("ssh.form.authentication")),
                    )
                    .child(self.render_connection_field(
                        self.i18n.t("ssh.form.username"),
                        &username,
                        self.i18n.t("ssh.form.username"),
                        NewConnectionField::Username,
                        false,
                        cx,
                    ))
                    .child(self.render_connection_secret_field(
                        self.i18n.t("ssh.form.password"),
                        self.i18n.t("ssh.form.password"),
                        NewConnectionField::Password,
                        cx,
                    ))
                    .child(self.render_connection_checkbox(
                        self.i18n.t("ssh.form.save_password"),
                        save_password,
                        |form| form.save_password = !form.save_password,
                        cx,
                    ))
                    .child(self.render_connection_checkbox(
                        self.i18n.t("modals.new_connection.ftp_tls"),
                        tls,
                        |form| form.ftp_tls = !form.ftp_tls,
                        cx,
                    ));

                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(self.tokens.metrics.modal_section_gap))
                    .child(basic)
                    .child(authentication)
                    .when(shows_icon_field, |content| {
                        content.child(self.render_connection_appearance_fields(
                            &icon,
                            &color,
                            &bg_color,
                            icon_picker_expanded,
                            false,
                            cx,
                        ))
                    })
                    .child(self.render_connection_notes_fields(&notes, cx))
                    .into_any_element()
            }
            ConnectionFormTab::Terminal => div()
                .flex()
                .flex_col()
                .gap(px(self.tokens.metrics.modal_section_gap))
                .child(self.render_connection_field(
                    self.i18n.t("modals.new_connection.ftp_initial_path"),
                    &path,
                    "/".into(),
                    NewConnectionField::InitialRemotePath,
                    false,
                    cx,
                ))
                .child(self.render_connection_field(
                    self.i18n.t("modals.new_connection.ftp_timeout"),
                    &timeout,
                    "30".into(),
                    NewConnectionField::ConnectTimeoutSeconds,
                    false,
                    cx,
                ))
                .child(self.render_upstream_proxy_policy_section(false, cx))
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }
}
